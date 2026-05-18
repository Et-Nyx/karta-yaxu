import { writable } from 'svelte/store';

export const pageSource = writable<string | null>(null);
