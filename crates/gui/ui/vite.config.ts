import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  root: import.meta.dirname,
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/target/**', '**/.git/**'] },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
