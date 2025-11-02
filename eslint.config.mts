import css from "@eslint/css";
import {defineConfig} from "eslint/config";
import json from "@eslint/json";
import markdown from "@eslint/markdown";
import tseslint from "typescript-eslint";


export default defineConfig([
    {
        ignores: [
            "**/.idea/**",
            "**/@girs/**",
            "**/build/**",
            "**/node_modules/**",
            "**/*.md",
            "**/package.json",
            "**/package-lock.json",
            "**/tsconfig.json",
        ]
    },
    tseslint.configs.recommended,
    {
        files: ["**/*.{ts,tsx,js,mjs}"],
        rules: {
            "@typescript-eslint/no-unused-vars": [
                "warn",
                {
                    argsIgnorePattern: "^_",
                    varsIgnorePattern: "^_",
                },
            ],
        },
    },
    {
        language: "json/json",
        plugins: {json},
        extends: ["json/recommended"],
        files: ["**/*.json"],
    },
    {
        language: "json/jsonc",
        plugins: {json},
        extends: ["json/recommended"],
        files: ["**/*.jsonc"],
    },
    {
        language: "json/json5",
        plugins: {json},
        extends: ["json/recommended"],
        files: ["**/*.json5"],
    },
    {
        language: "markdown/gfm",
        plugins: {markdown},
        extends: ["markdown/recommended"],
        files: ["**/*.md"],
    },
    {
        language: "css/css",
        plugins: {css},
        extends: ["css/recommended"],
        files: ["**/*.css"],
    },
]);
