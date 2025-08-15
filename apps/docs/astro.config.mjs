import starlight from '@astrojs/starlight'
import { defineConfig } from 'astro/config'
import starlightOpenAPI, { openAPISidebarGroups } from 'starlight-openapi'

// https://astro.build/config
export default defineConfig({
  site: 'https://docs.modrinth.com',
  integrations: [
    starlight({
      title: 'BBSMC 文档',
      favicon: '/favicon.ico',
      editLink: {
        baseUrl: 'https://github.com/bbsmc/app/edit/main/apps/docs/',
      },
      social: {
        github: 'https://github.com/bbsmc/app',
      },
      logo: {
        light: './src/assets/light-logo.svg',
        dark: './src/assets/dark-logo.svg',
        replacesTitle: true,
      },
      customCss: [
        './src/styles/modrinth.css',
      ],
      defaultLocale: 'zh-CN',
      locales: {
        'zh-CN': {
          label: '简体中文',
        },
      },
      plugins: [
        // Generate the OpenAPI documentation pages.
        starlightOpenAPI([
          {
            base: 'api',
            label: 'BBSMC API',
            schema: './public/openapi.yaml',
          },
        ]),
      ],
      sidebar: [
        {
          label: '为 BBSMC 贡献',
          autogenerate: { directory: 'contributing' },
        },
        {
          label: '指南',
          autogenerate: { directory: 'guide' },
        },
        // Add the generated sidebar group to the sidebar.
        ...openAPISidebarGroups,
      ],
    }),
  ],
})