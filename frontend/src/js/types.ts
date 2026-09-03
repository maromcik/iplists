export enum AppErrorKind {
    InternalServerError = "InternalServerError",
    NotFound = "NotFound",
    BadRequest = "BadRequest",
    TemplatingError = "TemplatingError",
    IdentityError = "IdentityError",
    SessionError = "SessionError",
    CookieError = "CookieError",
    FileError = "FileError",
    DataFileLoadError = "DataFileLoadError",
    Unauthorized = "Unauthorized",
    ParseError = "ParseError",
    EnvVarError = "EnvVarError",
    ConfigError = "ConfigError",
    RequestError = "RequestError",
    FmtError = "FmtError",
    DatetimeError = "DatetimeError",
    SchedulerError = "SchedulerError",
    Unknown = "Unknown",
}

export interface ApiError {
    code: number;
    kind: AppErrorKind;
    description: string;
}

export interface Location {
    name: string;
    code: string;
    continent: string;
}

export type ActiveLocation = { region: string } | { alpha2: string };

export interface CombinedIpRange {
    ip: string;
    network: string;
    asn: number;
    isp: string;
    location: Location;
}

export enum StatusCode {
    Ok = 0,
    Warning = 1,
    Error = 2,
    Disaster = 3,
}

export interface StatusInfo {
    status_code: StatusCode;
    status_meaning: string;
    message: string;
}

export interface UpdateStatus {
    last_update: string | null;
    next_update: string | null;
}

export interface ComponentStatus {
    status: StatusInfo;
    update: UpdateStatus;
}


export interface AppStatus {
    overall: StatusInfo;
    db: StatusInfo;
    locations: ComponentStatus;
    asns: ComponentStatus;
    geo: ComponentStatus;
    blocklist: ComponentStatus;
}
