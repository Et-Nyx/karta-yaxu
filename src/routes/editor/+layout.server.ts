import { requireEditor } from '$lib/auth/guards';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ locals }) => {
  requireEditor(locals);
  return {};
};
