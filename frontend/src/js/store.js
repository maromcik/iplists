import { writable } from 'svelte/store';
export const user = writable('');
export const activeLocation = writable('');
export const locationType = writable('');