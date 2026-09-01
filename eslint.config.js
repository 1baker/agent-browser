import js from '@eslint/js';
import globals from 'globals';

export default [
  {
    ignores: [
      '.codegraph/**',
      'bin/**',
      'cli/**',
      'coverage/**',
      'docs/**',
      'node_modules/**',
      'packages/**',
      'test-results/**',
    ],
  },
  {
    files: ['eslint.config.js', 'scripts/**/*.js'],
    ...js.configs.recommended,
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
      globals: globals.node,
    },
    linterOptions: {
      reportUnusedDisableDirectives: 'error',
    },
    rules: {
      ...js.configs.recommended.rules,
      'no-empty': ['error', { allowEmptyCatch: true }],
      // Recovery and test scripts intentionally assign sentinels before cleanup boundaries.
      'no-useless-assignment': 'off',
      // CLI scripts replace low-level failures with stable operator-facing messages.
      'preserve-caught-error': 'off',
    },
  },
];
