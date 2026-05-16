import { error, fail, redirect } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { natures } from '$lib/db/schema/items';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
import type { PageServerLoad, Actions } from './$types';

const STAT_VALUES = ['attack', 'defense', 'special-attack', 'special-defense', 'speed'] as const;
const FLAVOR_VALUES = ['spicy', 'sour', 'sweet', 'bitter', 'dry'] as const;

const slugRegex = /^[a-z0-9-]+$/;

const UpdateSchema = z.object({
  name:          z.string().trim().min(1).max(50),
  slug:          z.string().trim().min(1).max(50).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  increasedStat: z.enum(STAT_VALUES).optional().or(z.literal('').transform(() => undefined)),
  decreasedStat: z.enum(STAT_VALUES).optional().or(z.literal('').transform(() => undefined)),
  likesFlavor:   z.enum(FLAVOR_VALUES).optional().or(z.literal('').transform(() => undefined)),
  hatesFlavor:   z.enum(FLAVOR_VALUES).optional().or(z.literal('').transform(() => undefined)),
});

export const load: PageServerLoad = async ({ params }) => {
  const [row] = await db.select().from(natures).where(eq(natures.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Nature not found');
  return { nature: row };
};

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateSchema.safeParse(form);

    if (!parsed.success) {
      return fail(400, {
        updateError: parsed.error.issues[0]?.message ?? 'Inválido',
        values: form,
      });
    }

    const data = parsed.data;
    const [current] = await db.select().from(natures).where(eq(natures.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Nature not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: natures.id })
        .from(natures).where(eq(natures.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    const newRow = {
      name:          data.name,
      slug:          data.slug,
      increasedStat: data.increasedStat ?? null,
      decreasedStat: data.decreasedStat ?? null,
      likesFlavor:   data.likesFlavor ?? null,
      hatesFlavor:   data.hatesFlavor ?? null,
      source:        bumpSource(current.source),
    };

    await db.update(natures).set(newRow).where(eq(natures.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'natures', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/natures/${data.slug}`);
    return { saved: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(natures).where(eq(natures.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Nature not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas registros edited podem ser re-sourceados' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(natures).set({ source: 'pokeapi' }).where(eq(natures.id, current.id));
    try {
      await reimportEntity('natures', current.pokeapiId);
    } catch (err) {
      await db.update(natures).set({ source: 'edited' }).where(eq(natures.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'natures', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(natures).where(eq(natures.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Nature not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas registros source=custom podem ser deletados' });
    }

    await db.delete(natures).where(eq(natures.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'natures', current.id, current, null);
    }

    throw redirect(303, '/editor/natures');
  },
};
