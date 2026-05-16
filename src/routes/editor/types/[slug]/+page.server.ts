import { error, fail, redirect } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { types } from '$lib/db/schema/core-data';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
import type { PageServerLoad, Actions } from './$types';

const DMG_CLASS_VALUES = ['physical', 'special', 'status'] as const;

const slugRegex = /^[a-z0-9-]+$/;
const hexRegex = /^#[0-9A-Fa-f]{6}$/;

const UpdateSchema = z.object({
  name:            z.string().trim().min(1).max(50),
  slug:            z.string().trim().min(1).max(50).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  hexColor:        z.string().trim().regex(hexRegex, 'hexColor: formato #RRGGBB').optional().or(z.literal('').transform(() => undefined)),
  iconUrl:         z.string().trim().max(255).optional().or(z.literal('').transform(() => undefined)),
  moveDamageClass: z.enum(DMG_CLASS_VALUES).optional().or(z.literal('').transform(() => undefined)),
  generationId:    z.coerce.number().int().positive().optional().or(z.literal('').transform(() => undefined)),
});

export const load: PageServerLoad = async ({ params }) => {
  const [row] = await db.select().from(types).where(eq(types.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Type not found');
  return { type: row };
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
    const [current] = await db.select().from(types).where(eq(types.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Type not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: types.id })
        .from(types).where(eq(types.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    const newRow = {
      name:            data.name,
      slug:            data.slug,
      hexColor:        data.hexColor ?? null,
      iconUrl:         data.iconUrl ?? null,
      moveDamageClass: data.moveDamageClass ?? null,
      generationId:    data.generationId ?? null,
      source:          bumpSource(current.source),
    };

    await db.update(types).set(newRow).where(eq(types.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'types', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/types/${data.slug}`);
    return { saved: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(types).where(eq(types.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Type not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas registros edited podem ser re-sourceados' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(types).set({ source: 'pokeapi' }).where(eq(types.id, current.id));
    try {
      await reimportEntity('types', current.pokeapiId);
    } catch (err) {
      await db.update(types).set({ source: 'edited' }).where(eq(types.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'types', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(types).where(eq(types.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Type not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas registros source=custom podem ser deletados' });
    }

    await db.delete(types).where(eq(types.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'types', current.id, current, null);
    }

    throw redirect(303, '/editor/types');
  },
};
