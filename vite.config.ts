import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import obfuscator from 'rollup-plugin-obfuscator';
import { COMMUNITY_OBFUSCATOR_OPTIONS } from './vite.obfuscation';

export default defineConfig(({ mode }) => {
  const isCommunity = mode === 'community';

  return {
    plugins: [
      tailwindcss(),
      react(),
      isCommunity &&
        obfuscator({
          global: true,
          options: COMMUNITY_OBFUSCATOR_OPTIONS,
        }),
    ].filter(Boolean),
    resolve: {
      dedupe: ['react', 'react-dom'],
    },
    clearScreen: false,
    server: {
      host: '127.0.0.1',
      port: 1423,
      strictPort: true,
    },
    build: isCommunity
      ? {
          sourcemap: false,
          minify: true,
          cssMinify: true,
          target: 'es2020',
          rollupOptions: {
            output: {
              codeSplitting: false,
            },
          },
        }
      : undefined,
  };
});
