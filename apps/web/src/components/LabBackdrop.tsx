/** Soft illustrated lab atmosphere for the welcome hero (CSS-only, Firefox-friendly). */
export function LabBackdrop() {
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
      <svg
        className="animate-float absolute -right-8 top-24 h-[22rem] w-[22rem] opacity-90 sm:right-8 sm:top-16 lg:right-16"
        viewBox="0 0 320 320"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          <linearGradient id="glass" x1="80" y1="40" x2="240" y2="280" gradientUnits="userSpaceOnUse">
            <stop stopColor="#F7FFFE" stopOpacity="0.95" />
            <stop offset="1" stopColor="#7FB8B2" stopOpacity="0.55" />
          </linearGradient>
          <linearGradient id="liquid" x1="120" y1="180" x2="200" y2="280" gradientUnits="userSpaceOnUse">
            <stop stopColor="#C45C26" stopOpacity="0.85" />
            <stop offset="1" stopColor="#E8A07A" stopOpacity="0.5" />
          </linearGradient>
        </defs>
        <path
          d="M118 48h84l8 28h-100l8-28z"
          fill="#0F2A2E"
          opacity="0.85"
        />
        <path
          d="M126 76h68l34 168c2 12-7 24-20 24H112c-13 0-22-12-20-24l34-168z"
          fill="url(#glass)"
          stroke="#1A4F55"
          strokeWidth="3"
        />
        <path
          d="M118 210c18-18 66-18 84 0v46c0 10-8 18-18 18h-48c-10 0-18-8-18-18v-46z"
          fill="url(#liquid)"
        />
        <circle cx="148" cy="236" r="6" fill="#FFF6F0" opacity="0.55" />
        <circle cx="176" cy="248" r="4" fill="#FFF6F0" opacity="0.4" />
        <path d="M210 120c28-6 46 18 34 40" stroke="#1A4F55" strokeWidth="2" strokeLinecap="round" opacity="0.35" />
      </svg>

      <div
        className="absolute bottom-0 left-0 right-0 h-40 opacity-40"
        style={{
          background:
            'linear-gradient(90deg, transparent, rgba(26,79,85,0.15), transparent)',
        }}
      />
    </div>
  )
}
