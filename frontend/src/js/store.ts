import { writable, type Writable } from 'svelte/store';
import type { ActiveLocation } from './types';

function createPersistedTokenStore(): Writable<string> {
    const stored = typeof localStorage !== 'undefined'
        ? localStorage.getItem('iplists_token')
        : null;
    const store = writable<string>(stored || '');

    store.subscribe((value) => {
        if (typeof localStorage !== 'undefined') {
            localStorage.setItem('iplists_token', value);
        }
    });

    return store;
}

export const token: Writable<string> = createPersistedTokenStore();
export const user: Writable<string> = writable('');
export const activeLocation: Writable<ActiveLocation | null> = writable(null);
export const locationType: Writable<string> = writable('');
