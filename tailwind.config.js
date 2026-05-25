/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        // Soundvault brand tokens — desaturated cyan-violet on near-black.
        // Designed against the dark theme; light tokens mirror with reduced saturation.
        ink: {
          50: "#f4f4f6",
          100: "#e7e7eb",
          200: "#c8c8d2",
          300: "#9a9aab",
          400: "#6f6f82",
          500: "#52525f",
          600: "#3a3a45",
          700: "#272730",
          800: "#1a1a22",
          900: "#11111a",
          950: "#08080d",
        },
        accent: {
          // Soft indigo-violet (Linear-ish, slightly desaturated).
          50: "#eef0ff",
          100: "#dde1ff",
          200: "#c0c7ff",
          300: "#9ba4ff",
          400: "#7a83f5",
          500: "#6470e8",
          600: "#4f5acd",
          700: "#3f48a3",
          800: "#2c3378",
          900: "#1c204d",
        },
        glow: {
          // Cool cyan secondary used sparingly for highlights.
          400: "#7cd5e6",
          500: "#52bdd1",
        },
      },
      fontFamily: {
        sans: [
          "Inter",
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Roboto",
          "sans-serif",
        ],
        mono: [
          "JetBrains Mono",
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Consolas",
          "monospace",
        ],
      },
      borderRadius: {
        glass: "14px",
      },
      boxShadow: {
        glass:
          "0 1px 0 rgba(255,255,255,0.04) inset, 0 8px 32px rgba(0,0,0,0.45), 0 2px 6px rgba(0,0,0,0.35)",
        "glass-light":
          "0 1px 0 rgba(255,255,255,0.6) inset, 0 8px 32px rgba(15,15,30,0.10), 0 2px 6px rgba(15,15,30,0.06)",
        glow: "0 0 0 1px rgba(122,131,245,0.35), 0 6px 28px rgba(100,112,232,0.35)",
      },
      keyframes: {
        "pulse-soft": {
          "0%, 100%": { opacity: "0.55" },
          "50%": { opacity: "1" },
        },
        "rise-in": {
          "0%": { transform: "translateY(8px)", opacity: "0" },
          "100%": { transform: "translateY(0)", opacity: "1" },
        },
        "fade-in": {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
      },
      animation: {
        "pulse-soft": "pulse-soft 2.4s ease-in-out infinite",
        "rise-in": "rise-in 280ms cubic-bezier(0.2, 0.7, 0.2, 1) both",
        "fade-in": "fade-in 220ms ease-out both",
      },
    },
  },
  plugins: [],
};
