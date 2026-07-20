import { get } from 'svelte/store';
import { token } from './store';

export function getAuthToken(): string {
    return get(token);
}

export function getAuthHeaders(): Record<string, string> {
    const t = getAuthToken();
    return t ? { Authorization: `Bearer ${t}` } : {};
}

export async function apiFetch(url: string, init: RequestInit = {}): Promise<Response> {
    return fetch(url, {
        ...init,
        headers: {
            ...getAuthHeaders(),
            ...(init.headers || {}),
        },
    });
}
