export type Source = 'pokeapi' | 'edited' | 'custom';

export function bumpSource(current: Source | string | null | undefined): Source {
  if (current === 'custom') return 'custom';
  return 'edited';
}
