import { fail, redirect } from '@sveltejs/kit';
import { asc, count, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { abilities } from '$lib/db/schema/core-data';
import { recordEdit } from '$lib/editor/audit';
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

const CreateSchema = z.object({
  name:         z.string().trim().min(1).max(50),
  slug:         z.string().trim().min(1).max(50).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  isMainSeries: boolField,
  generationId: intOrNull,
});

export const load: PageServerLoad = async () => {
  const rows = await db.select().from(abilities).orderBy(asc(abilities.id));
  const [{ total }] = await db.select({ total: count() }).from(abilities);
  return { abilities: rows, total };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = CreateSchema.safeParse(form);

    if (!parsed.success) {
      return fail(400, {
        createError: parsed.error.issues[0]?.message ?? 'Inválido',
        createValues: form,
      });
    }

    const data = parsed.data;
    const [existing] = await db.select({ id: abilities.id })
      .from(abilities).where(eq(abilities.slug, data.slug)).limit(1);
    if (existing) {
      return fail(400, { createError: 'slug já em uso', createValues: form });
    }

    const [inserted] = await db.insert(abilities).values({
      name:         data.name,
      slug:         data.slug,
      isMainSeries: data.isMainSeries,
      generationId: data.generationId ?? null,
      source:       'custom',
      pokeapiId:    null,
    }).returning({ id: abilities.id });

    if (locals.user) {
      await recordEdit(locals.user.id, 'abilities', inserted.id, null, data);
    }

    throw redirect(303, `/editor/abilities/${data.slug}`);
  },
};
