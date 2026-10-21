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
    search: ["M11 3a8 8 0 1 0 0 16a8 8 0 1 0 0-16", "m21 21-4.3-4.3"],
    sort: ["M3 6h18M6 12h12M10 18h4"],
    message: ["M7.9 20A9 9 0 1 0 4 16.1L2 22z"],
    bell: ["M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9", "M10.3 21a1.9 1.9 0 0 0 3.4 0"],
    mail: ["M4 4h16a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z", "m22 7-10 6L2 7"],
    inbox: ["M22 12h-6l-2 3h-4l-2-3H2", "M5.5 5.1 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.5-6.9A2 2 0 0 0 16.8 4H7.2a2 2 0 0 0-1.7 1.1z"],
    star: ["m12 2 3.1 6.3 6.9 1-5 4.9 1.2 6.8-6.2-3.2-6.2 3.2L7 14.2 2 9.3l6.9-1z"],
    trash: ["M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"],
    reply: ["m9 17-5-5 5-5", "M20 18v-2a4 4 0 0 0-4-4H4"],
    replyAll: ["m7 17-5-5 5-5", "m12 17-5-5 5-5", "M22 18v-2a4 4 0 0 0-4-4H7"],
    forward: ["m15 17 5-5-5-5", "M4 18v-2a4 4 0 0 1 4-4h12"],
    paperclip: ["m21.4 11-8.8 8.8a6 6 0 0 1-8.5-8.5l8.8-8.8a4 4 0 0 1 5.7 5.7l-8.8 8.8a2 2 0 0 1-2.8-2.8l8.1-8.1"],
    edit: ["M12 20h9", "M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z"],
    undo: ["M3 7v6h6", "M21 17a9 9 0 0 0-15-6.7L3 13"],
    eyeOff: ["m2 2 20 20", "M6.7 6.7C3.8 8.5 2 12 2 12s3.6 7 10 7c1.9 0 3.6-.6 5-1.4M10.7 5.1A9.8 9.8 0 0 1 12 5c6.4 0 10 7 10 7s-.7 1.4-2.1 3"],
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
