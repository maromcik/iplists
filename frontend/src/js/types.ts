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
