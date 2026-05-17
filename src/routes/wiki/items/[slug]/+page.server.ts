import { db } from '$lib/db';
import { eq, asc, and } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
  const { slug } = params;

  // 1. Fetch item
  const iList = await db.select()
    .from(schema.items)
    .where(eq(schema.items.slug, slug))
    .limit(1);

  if (iList.length === 0) throw error(404, 'Item not found');

  const item = iList[0];

  // 2. Fetch English language id
  const langList = await db.select({ id: schema.languages.id })
    .from(schema.languages)
    .where(eq(schema.languages.name, 'en'))
    .limit(1);

  const englishId = langList[0]?.id ?? null;

  // 3. Flavor texts with version group name
  const flavorTexts = englishId
    ? await db.select({
        flavorText: schema.itemFlavorTexts.flavorText,
        versionGroupId: schema.itemFlavorTexts.versionGroupId,
        versionGroupName: schema.versionGroups.name,
      })
      .from(schema.itemFlavorTexts)
      .innerJoin(schema.versionGroups, eq(schema.itemFlavorTexts.versionGroupId, schema.versionGroups.id))
      .where(and(eq(schema.itemFlavorTexts.itemId, item.id), eq(schema.itemFlavorTexts.languageId, englishId)))
      .orderBy(asc(schema.versionGroups.order))
    : [];

  // 4. Effect (English)
  const effectList = englishId
    ? await db.select({
        effect: schema.itemEffects.effect,
        shortEffect: schema.itemEffects.shortEffect,
      })
      .from(schema.itemEffects)
      .where(and(eq(schema.itemEffects.itemId, item.id), eq(schema.itemEffects.languageId, englishId)))
      .limit(1)
    : [];

  const effect = effectList[0] ?? null;

  return { item, flavorTexts, effect };
};
