import { writable } from 'svelte/store';
import { browser } from '$app/environment';

const STORAGE_KEY = 'karta_selected_pokedex';

const initialValue = browser ? window.localStorage.getItem(STORAGE_KEY) || '' : '';

export const selectedPokedexSlug = writable<string>(initialValue);

if (browser) {
  selectedPokedexSlug.subscribe((value) => {
    if (value) {
      window.localStorage.setItem(STORAGE_KEY, value);
    } else {
      window.localStorage.removeItem(STORAGE_KEY);
    }
  });
}
