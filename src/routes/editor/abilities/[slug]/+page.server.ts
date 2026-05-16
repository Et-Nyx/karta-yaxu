import { error, fail, redirect } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { abilities } from '$lib/db/schema/core-data';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const boolField = z.preprocess(
  (v) => v === 'true' || v === 'on' || v === true,
  z.boolean(),
);

const intOrNull = z.preprocess(
  (v) => {
    if (v === '' || v === null || v === undefined) return null;
    const n = Number(v);
    return Number.isFinite(n) ? n : v;
  },
  z.number().int().positive().nullable(),
);

const UpdateSchema = z.object({
  name:         z.string().trim().min(1).max(50),
  slug:         z.string().trim().min(1).max(50).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  isMainSeries: boolField,
  generationId: intOrNull,
});

export const load: PageServerLoad = async ({ params }) => {
  const [row] = await db.select().from(abilities).where(eq(abilities.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Ability not found');
  return { ability: row };
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
    const [current] = await db.select().from(abilities).where(eq(abilities.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Ability not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: abilities.id })
        .from(abilities).where(eq(abilities.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    const newRow = {
      name:         data.name,
      slug:         data.slug,
      isMainSeries: data.isMainSeries,
      generationId: data.generationId ?? null,
      source:       bumpSource(current.source),
    };

    await db.update(abilities).set(newRow).where(eq(abilities.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'abilities', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/abilities/${data.slug}`);
    return { saved: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(abilities).where(eq(abilities.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Ability not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas registros edited podem ser re-sourceados' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(abilities).set({ source: 'pokeapi' }).where(eq(abilities.id, current.id));
    try {
      await reimportEntity('abilities', current.pokeapiId);
    } catch (err) {
      await db.update(abilities).set({ source: 'edited' }).where(eq(abilities.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'abilities', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(abilities).where(eq(abilities.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Ability not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas registros source=custom podem ser deletados' });
    }

    await db.delete(abilities).where(eq(abilities.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'abilities', current.id, current, null);
    }

    throw redirect(303, '/editor/abilities');
  },
};
