import { fail, redirect } from '@sveltejs/kit';
import { asc, count, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { types } from '$lib/db/schema/core-data';
import { recordEdit } from '$lib/editor/audit';
import type { PageServerLoad, Actions } from './$types';

const DMG_CLASS_VALUES = ['physical', 'special', 'status'] as const;

const slugRegex = /^[a-z0-9-]+$/;
const hexRegex = /^#[0-9A-Fa-f]{6}$/;

const CreateSchema = z.object({
  name:            z.string().trim().min(1).max(50),
  slug:            z.string().trim().min(1).max(50).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  hexColor:        z.string().trim().regex(hexRegex, 'hexColor: formato #RRGGBB').optional().or(z.literal('').transform(() => undefined)),
  iconUrl:         z.string().trim().max(255).optional().or(z.literal('').transform(() => undefined)),
  moveDamageClass: z.enum(DMG_CLASS_VALUES).optional().or(z.literal('').transform(() => undefined)),
  generationId:    z.coerce.number().int().positive().optional().or(z.literal('').transform(() => undefined)),
});

export const load: PageServerLoad = async () => {
  const rows = await db.select().from(types).orderBy(asc(types.slug));
  const [{ total }] = await db.select({ total: count() }).from(types);
  return { types: rows, total };
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
    const [existing] = await db.select({ id: types.id })
      .from(types).where(eq(types.slug, data.slug)).limit(1);
    if (existing) {
      return fail(400, { createError: 'slug já em uso', createValues: form });
    }

    const [inserted] = await db.insert(types).values({
      name:            data.name,
      slug:            data.slug,
      hexColor:        data.hexColor ?? null,
      iconUrl:         data.iconUrl ?? null,
      moveDamageClass: data.moveDamageClass ?? null,
      generationId:    data.generationId ?? null,
      source:          'custom',
      pokeapiId:       null,
    }).returning({ id: types.id });

    if (locals.user) {
      await recordEdit(locals.user.id, 'types', inserted.id, null, data);
    }

    throw redirect(303, `/editor/types/${data.slug}`);
  },
};
