import { writable, type Writable } from 'svelte/store';
import type { ActiveLocation } from './types';
export const user: Writable<string> = writable('');
export const activeLocation: Writable<ActiveLocation | null> = writable(null);
export const locationType: Writable<string> = writable('');