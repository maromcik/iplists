export interface Location {
    name: string;
    alpha2: string;
    continent: string;
}

export type ActiveLocation = { region: string } | { alpha2: string };

export interface IpLocationRange {
    start: string;
    end: string;
    country_name: string;
    country_alpha2: string;
    continent: string;
}

export interface IpAsnRange {
    start: string;
    end: string;
    asn: number;
    isp: string;
}
