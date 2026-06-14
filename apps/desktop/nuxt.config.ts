export default defineNuxtConfig({
  srcDir: 'frontend/',
  css: ['~/assets/css/main.css'],
  experimental: {
    appManifest: false
  },
  app: {
    head: {
      title: 'Codex Skill Manager',
      meta: [{ name: 'viewport', content: 'width=device-width, initial-scale=1' }]
    }
  },
  compatibilityDate: '2025-01-06',
  devtools: { enabled: true }
});
