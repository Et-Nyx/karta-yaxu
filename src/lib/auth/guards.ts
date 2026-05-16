import { redirect, error } from '@sveltejs/kit';

export function requireAuth(locals: App.Locals) {
  if (!locals.user) throw redirect(302, '/trainer/login');
}

export function requireEditor(locals: App.Locals) {
  if (!locals.user) throw redirect(302, '/trainer/login');
  if (locals.user.role !== 'editor') throw error(403, 'Acesso negado');
}
