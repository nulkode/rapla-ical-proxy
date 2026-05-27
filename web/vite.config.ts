import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  base: '/web/',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    sourcemap: true,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        public: resolve(__dirname, 'public.html'),
      },
    },
  },
  esbuild: {
    tsconfigRaw: '{}',
  },
});
