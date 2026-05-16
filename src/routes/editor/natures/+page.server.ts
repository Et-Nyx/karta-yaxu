import { fail, redirect } from '@sveltejs/kit';
import { asc, count, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { natures } from '$lib/db/schema/items';
import { recordEdit } from '$lib/editor/audit';
import type { PageServerLoad, Actions } from './$types';

const STAT_VALUES = ['attack', 'defense', 'special-attack', 'special-defense', 'speed'] as const;
const FLAVOR_VALUES = ['spicy', 'sour', 'sweet', 'bitter', 'dry'] as const;

const slugRegex = /^[a-z0-9-]+$/;

const CreateSchema = z.object({
  name:          z.string().trim().min(1).max(50),
  slug:          z.string().trim().min(1).max(50).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  increasedStat: z.enum(STAT_VALUES).optional().or(z.literal('').transform(() => undefined)),
  decreasedStat: z.enum(STAT_VALUES).optional().or(z.literal('').transform(() => undefined)),
  likesFlavor:   z.enum(FLAVOR_VALUES).optional().or(z.literal('').transform(() => undefined)),
  hatesFlavor:   z.enum(FLAVOR_VALUES).optional().or(z.literal('').transform(() => undefined)),
});

export const load: PageServerLoad = async () => {
  const rows = await db.select().from(natures).orderBy(asc(natures.slug));
  const [{ total }] = await db.select({ total: count() }).from(natures);
  return { natures: rows, total };
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
    const [existing] = await db.select({ id: natures.id })
      .from(natures).where(eq(natures.slug, data.slug)).limit(1);
    if (existing) {
      return fail(400, { createError: 'slug já em uso', createValues: form });
    }

    const [inserted] = await db.insert(natures).values({
      name:          data.name,
      slug:          data.slug,
      increasedStat: data.increasedStat ?? null,
      decreasedStat: data.decreasedStat ?? null,
      likesFlavor:   data.likesFlavor ?? null,
      hatesFlavor:   data.hatesFlavor ?? null,
      source:        'custom',
      pokeapiId:     null,
    }).returning({ id: natures.id });

    if (locals.user) {
      await recordEdit(locals.user.id, 'natures', inserted.id, null, data);
    }

    throw redirect(303, `/editor/natures/${data.slug}`);
  },
};
