import {
    StatusCode,
    type AppStatus,
    type ComponentStatus,
    type StatusInfo,
    type UpdateStatus,
} from "./types";

const VALID_CODES: number[] = [
    StatusCode.Ok,
    StatusCode.Warning,
    StatusCode.Error,
    StatusCode.Disaster,
];

function bad(path: string, expected: string, value: unknown): Error {
    return new Error(
        "invalid /api/status payload: " +
            path +
            " should be " +
            expected +
            ", got " +
            JSON.stringify(value),
    );
}

function expectRecord(value: unknown, path: string): Record<string, unknown> {
    if (typeof value !== "object" || value === null) {
        throw bad(path, "an object", value);
    }
    return value as Record<string, unknown>;
}

function expectString(value: unknown, path: string): string {
    if (typeof value !== "string") throw bad(path, "a string", value);
    return value;
}

function expectNullableString(value: unknown, path: string): string | null {
    if (value === null) return null;
    return expectString(value, path);
}

function parseStatusCode(value: unknown, path: string): StatusCode {
    if (typeof value !== "number" || !VALID_CODES.includes(value)) {
        throw bad(path, "an integer status code between 0 and 3", value);
    }
    return value as StatusCode;
}

function parseStatusInfo(value: unknown, path: string): StatusInfo {
    const record = expectRecord(value, path);
    return {
        status_code: parseStatusCode(record.status_code, path + ".status_code"),
        status_meaning: expectString(record.status_meaning, path + ".status_meaning"),
        message: expectString(record.message, path + ".message"),
    };
}

function parseUpdateStatus(value: unknown, path: string): UpdateStatus {
    const record = expectRecord(value, path);
    return {
        last_update: expectNullableString(record.last_update, path + ".last_update"),
        next_update: expectNullableString(record.next_update, path + ".next_update"),
    };
}

function parseComponentStatus(value: unknown, path: string): ComponentStatus {
    const record = expectRecord(value, path);
    return {
        status: parseStatusInfo(record.status, path + ".status"),
        update: parseUpdateStatus(record.update, path + ".update"),
    };
}

export function parseAppStatus(data: unknown): AppStatus {
    const record = expectRecord(data, "$");
    return {
        overall: parseStatusInfo(record.overall, "$.overall"),
        locations: parseComponentStatus(record.locations, "$.locations"),
        asns: parseComponentStatus(record.asns, "$.asns"),
        geo: parseComponentStatus(record.geo, "$.geo"),
        blocklist: parseComponentStatus(record.blocklist, "$.blocklist"),
    };
}
