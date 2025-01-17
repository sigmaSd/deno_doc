import { type Config } from "npm:tailwindcss@3.4.0";

const TAG_PURPLE = "#7B61FF";
const TAG_CYAN = "#0CAFC6";

const extraColors = {
  "Function": "#056CF0",
  "Variable": "#7E57C0",
  "Class": "#20B44B",
  "Enum": "#22ABB0",
  "Interface": "#D2A064",
  "TypeAlias": "#A4478C",
  "Namespace": "#D25646",

  "new": TAG_PURPLE,
  "abstract": TAG_CYAN,
  "deprecated": "#DC2626", // red 600
  "writeonly": TAG_PURPLE,
  "readonly": TAG_PURPLE,
  "protected": TAG_PURPLE,
  "private": TAG_PURPLE,
  "optional": TAG_CYAN,
  "permissions": TAG_CYAN,
  "other": "#57534E", // stone 600
};

export default {
  content: [
    "./src/html/**/*.rs",
    "./src/html/templates/*.hbs",
  ],
  safelist: [
    {
      pattern: new RegExp(`^text-(${Object.keys(extraColors).join("|")})$`),
    },
    {
      pattern: new RegExp(`^bg-(${Object.keys(extraColors).join("|")})\/15$`),
    },
  ],
  theme: {
    extend: {
      colors: extraColors,
    },
  },
  darkMode: "class",
} as Config;
