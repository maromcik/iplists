<script lang="ts">
    import { StatusCode, type StatusInfo } from "../js/types";

    export let status: StatusInfo;

    // 0 = ok (green), 1 = warning (yellow), 2 = error (amber), 3 = unavailable (red)
    const COLORS: Record<StatusCode, string> = {
        [StatusCode.Ok]: "bg-green-600 text-white",
        [StatusCode.Warning]: "bg-yellow-400 text-gray-900",
        [StatusCode.Error]: "bg-amber-500 text-gray-900",
        [StatusCode.Unavailable]: "bg-red-600 text-white",
    };
</script>

<span
    class="inline-flex items-center gap-1.5 px-3 py-0.5 rounded-full text-xs font-semibold {COLORS[status.status_code]}"
    title={status.message}
>
    <i class="fas fa-circle text-[0.5rem]"></i>
    {status.status_meaning}
</span>
