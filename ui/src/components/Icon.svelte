<script lang="ts" module>
  // Strichsymbole im Stil von Lucide (24×24, Linienstärke 2).
  const icons: Record<string, string[]> = {
    today: ["M12 8a4 4 0 1 0 0 8a4 4 0 1 0 0-8", "M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M6.3 17.7l-1.4 1.4M19.1 4.9l-1.4 1.4"],
    courses: ["M2 4h6a4 4 0 0 1 4 4v13a3 3 0 0 0-3-3H2z", "M22 4h-6a4 4 0 0 0-4 4v13a3 3 0 0 1 3-3h7z"],
    timetable: ["M5 4h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z", "M16 2v4M8 2v4M3 10h18"],
    settings: ["M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"],
    refresh: ["M21 12a9 9 0 0 1-15.5 6.3L3 16M3 12a9 9 0 0 1 15.5-6.3L21 8", "M21 3v5h-5M3 21v-5h5"],
    file: ["M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z", "M14 2v6h6"],
    fileText: ["M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z", "M14 2v6h6M16 13H8M16 17H8"],
    folder: ["M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"],
    assign: ["M9 2h6a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1z", "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2", "m9 14 2 2 4-4"],
    forum: ["M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"],
    quiz: ["m3 17 2 2 4-4M3 7l2 2 4-4M13 6h8M13 12h8M13 18h8"],
    link: ["M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7", "M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7"],
    lock: ["M6 11h12a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2z", "M8 11V7a4 4 0 0 1 8 0v4"],
    check: ["M20 6 9 17l-5-5"],
    chevronDown: ["m6 9 6 6 6-6"],
    chevronLeft: ["m15 18-6-6 6-6"],
    chevronRight: ["m9 18 6-6-6-6"],
    external: ["M15 3h6v6M10 14 21 3M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"],
    users: ["M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2", "M9 3a4 4 0 1 0 0 8a4 4 0 1 0 0-8", "M22 21v-2a4 4 0 0 0-3-3.9M16 3.1a4 4 0 0 1 0 7.8"],
    chart: ["M3 3v18h18", "M18 17V9M13 17V5M8 17v-3"],
    play: ["M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20", "m10 8 6 4-6 4z"],
    book: ["M4 19.5A2.5 2.5 0 0 1 6.5 17H20V2H6.5A2.5 2.5 0 0 0 4 4.5z", "M4 19.5A2.5 2.5 0 0 0 6.5 22H20v-5"],
    page: ["M4 4h16v16H4z", "M8 9h8M8 13h8M8 17h5"],
    video: ["M2 6h14v12H2z", "m16 10 6-4v12l-6-4"],
    clock: ["M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18", "M12 7v5l3 2"],
    plus: ["M12 5v14M5 12h14"],
    x: ["M18 6 6 18M6 6l12 12"],
    alert: ["M12 3 2 21h20z", "M12 10v4M12 17.5v.5"],
    upload: ["M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4", "m17 8-5-5-5 5M12 3v12"],
    send: ["m22 2-7 20-4-9-9-4z", "M22 2 11 13"],
    logout: ["M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4", "m16 17 5-5-5-5M21 12H9"],
    puzzle: ["M4 7h3a2 2 0 1 0 4 0h3v3a2 2 0 1 1 0 4v3h-3a2 2 0 1 0-4 0H4z"],
  };
</script>

<script lang="ts">
  let {
    name,
    size = 20,
    stroke = 2,
    label,
    class: className = "",
  }: { name: string; size?: number; stroke?: number; label?: string; class?: string } = $props();

  const paths = $derived(icons[name] ?? icons.file!);
</script>

<svg
  class={className}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  role={label ? "img" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : "true"}
>
  {#each paths as d (d)}<path {d} />{/each}
</svg>

<style>
  svg {
    flex: none;
  }
</style>
