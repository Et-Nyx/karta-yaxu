import { fail, redirect } from '@sveltejs/kit';
import { asc, count, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { items } from '$lib/db/schema/items';
import { recordEdit } from '$lib/editor/audit';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const optionalInt = z
  .union([z.literal(''), z.coerce.number().int().min(0)])
  .optional()
  .transform((v) => (v === '' || v === undefined ? undefined : v));

const optionalText = z
  .string()
  .trim()
  .optional()
  .transform((v) => (v === '' || v === undefined ? undefined : v));

const CreateSchema = z.object({
  name:       z.string().trim().min(1).max(100),
  slug:       z.string().trim().min(1).max(100).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  cost:       optionalInt,
  flingPower: optionalInt,
  category:   optionalText,
  spriteUrl:  optionalText,
});

export const load: PageServerLoad = async () => {
  const rows = await db.select().from(items).orderBy(asc(items.slug));
  const [{ total }] = await db.select({ total: count() }).from(items);
  return { items: rows, total };
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
    const [existing] = await db.select({ id: items.id })
      .from(items).where(eq(items.slug, data.slug)).limit(1);
    if (existing) {
      return fail(400, { createError: 'slug já em uso', createValues: form });
    }

    const [inserted] = await db.insert(items).values({
      name:       data.name,
      slug:       data.slug,
      cost:       data.cost ?? null,
      flingPower: data.flingPower ?? null,
      category:   data.category ?? null,
      spriteUrl:  data.spriteUrl ?? null,
      source:     'custom',
      pokeapiId:  null,
    }).returning({ id: items.id });

    if (locals.user) {
      await recordEdit(locals.user.id, 'items', inserted.id, null, data);
    }

    throw redirect(303, `/editor/items/${data.slug}`);
  },
};
