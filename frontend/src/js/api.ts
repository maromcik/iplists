import { get } from 'svelte/store';
import { token } from './store';
import { AppErrorKind, type ApiError } from './types';

export function getAuthToken(): string {
    return get(token);
}

export function getAuthHeaders(): Record<string, string> {
    const t = getAuthToken();
    return t ? { Authorization: `Bearer ${t}` } : {};
}

export class ApiRequestError extends Error {
    response: ApiError;

    constructor(response: ApiError) {
        super(response.description);
        this.response = response;
        this.name = "ApiRequestError";
    }
}

function parseErrorVariant(body: Record<string, unknown>): { kind: AppErrorKind; description: string } {
    const keys = Object.keys(body);
    if (keys.length !== 1) {
        return {
            kind: AppErrorKind.Unknown,
            description: JSON.stringify(body),
        };
    }

    const variant = keys[0];
    const description = String(body[variant] ?? "");

    if (Object.values(AppErrorKind).includes(variant as AppErrorKind)) {
        return { kind: variant as AppErrorKind, description };
    }

    return { kind: AppErrorKind.Unknown, description: `${variant}: ${description}` };
}

async function parseError(response: Response): Promise<ApiError> {
    const contentType = response.headers.get("content-type") || "";
    if (contentType.includes("application/json")) {
        try {
            const body = await response.json();
            if (
                body &&
                typeof body === "object" &&
                "code" in body &&
                "error" in body &&
                typeof body.error === "object" &&
                body.error !== null
            ) {
                const parsed = parseErrorVariant(body.error as Record<string, unknown>);
                return {
                    code: Number(body.code),
                    kind: parsed.kind,
                    description: parsed.description,
                };
            }
        } catch {
            // Fall through to default.
        }
    }
    const text = await response.text();
    return {
        code: response.status,
        kind: AppErrorKind.Unknown,
        description: text || response.statusText,
    };
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

export async function apiFetchJson<T>(url: string, init: RequestInit = {}): Promise<T> {
    let response: Response;
    try {
        response = await apiFetch(url, init);
    } catch (err) {
        const message = err instanceof Error ? err.message : "Network error";
        throw new ApiRequestError({ code: 0, kind: AppErrorKind.Unknown, description: message });
    }

    if (!response.ok) {
        throw new ApiRequestError(await parseError(response));
    }

    return response.json() as Promise<T>;
}

export async function apiFetchText(url: string, init: RequestInit = {}): Promise<string> {
    let response: Response;
    try {
        response = await apiFetch(url, init);
    } catch (err) {
        const message = err instanceof Error ? err.message : "Network error";
        throw new ApiRequestError({ code: 0, kind: AppErrorKind.Unknown, description: message });
    }

    if (!response.ok) {
        throw new ApiRequestError(await parseError(response));
    }

    return response.text();
}
