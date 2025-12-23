import MarkdownIt from 'markdown-it'

import pkg from 'xss'

const { FilterXSS, whiteList, escapeAttrValue, safeAttrValue } = pkg as any
// import { escapeAttrValue, FilterXSS, safeAttrValue, whiteList } from 'xss'

export const configuredXss = new FilterXSS({
  whiteList: {
    ...whiteList,
    summary: [],
    h1: ['id'],
    h2: ['id'],
    h3: ['id'],
    h4: ['id'],
    h5: ['id'],
    h6: ['id'],
    kbd: ['id'],
    input: ['checked', 'disabled', 'type'],
    iframe: ['width', 'height', 'allowfullscreen', 'frameborder', 'start', 'end'],
    img: [...(whiteList.img || []), 'usemap', 'style', 'align'],
    map: ['name'],
    area: [...(whiteList.a || []), 'coords'],
    a: [...(whiteList.a || []), 'rel'],
    td: [...(whiteList.td || []), 'style'],
    th: [...(whiteList.th || []), 'style'],
    picture: [],
    source: ['media', 'sizes', 'src', 'srcset', 'type'],
    p: [...(whiteList.p || []), 'align'],
    div: [...(whiteList.p || []), 'align'],
  },
  css: {
    whiteList: {
      'image-rendering': /^pixelated$/,
      'text-align': /^center|left|right$/,
      float: /^left|right$/,
    },
  },
  onIgnoreTagAttr: (tag, name, value) => {
    // Allow iframes from acceptable sources
    if (tag === 'iframe' && name === 'src') {
      const allowedSources = [
        {
          url: /^https?:\/\/player\.bilibili\.com\/player\.html/,
          allowedParameters: ['aid', 'bvid', 'cid', 'p'],
        },
      ]

      const url = new URL(value.startsWith('//') ? 'https:' + value : value)

      for (const source of allowedSources) {
        if (source.url.test(url.href)) {
          // Remove all parameters not in the allowed list
          Array.from(url.searchParams.keys()).forEach((param) => {
            if (!source.allowedParameters.includes(param)) {
              url.searchParams.delete(param)
            }
          })
          return `${name}="${escapeAttrValue(url.toString())}"`
        }
      }
    }

    // For Highlight.JS
    if (name === 'class' && ['pre', 'code', 'span'].includes(tag)) {
      const allowedClasses: string[] = []
      for (const className of value.split(/\s/g)) {
        if (className.startsWith('hljs-') || className.startsWith('language-')) {
          allowedClasses.push(className)
        }
      }
      return `${name}="${escapeAttrValue(allowedClasses.join(' '))}"`
    }
  },

  safeAttrValue(tag, name, value, cssFilter) {
    if (
      (tag === 'img' || tag === 'video' || tag === 'audio' || tag === 'source') &&
      (name === 'src' || name === 'srcset') &&
      !value.startsWith('data:')
    ) {
      try {
        const url = new URL(value)

        if (url.hostname.includes('wsrv.nl')) {
          url.searchParams.delete('errorredirect')
        }

        const allowedHostnames = [
          'imgur.com',
          'i.imgur.com',
          'cdn-raw.modrinth.com',
          'cdn.modrinth.com',
          'cdn.bbsmc.net',
          'staging-cdn-raw.modrinth.com',
          'staging-cdn.modrinth.com',
          'github.com',
          'raw.githubusercontent.com',
          'img.shields.io',
          'i.postimg.cc',
          'wsrv.nl',
          'cf.way2muchnoise.eu',
          'bstats.org',
          'sm.ms',
        ]

        if (!allowedHostnames.includes(url.hostname)) {
          return safeAttrValue(
            tag,
            name,
            `https://wsrv.nl/?url=${encodeURIComponent(
              url.toString().replaceAll('&amp;', '&'),
            )}&n=-1`,
            cssFilter,
          )
        }
        return safeAttrValue(tag, name, url.toString(), cssFilter)
      } catch {
        /* empty */
      }
    }

    return safeAttrValue(tag, name, value, cssFilter)
  },
})

export const md = (options = {}) => {
  const md = new MarkdownIt('default', {
    html: true,
    linkify: true,
    breaks: false,
    ...options,
  })

  const defaultLinkOpenRenderer =
    md.renderer.rules.link_open ||
    function (tokens, idx, options, _env, self) {
      return self.renderToken(tokens, idx, options)
    }

  md.renderer.rules.link_open = function (tokens, idx, options, env, self) {
    const token = tokens[idx]
    const index = token.attrIndex('href')

    if (token.attrs && index !== -1) {
      const href = token.attrs[index][1]

      try {
        const url = new URL(href)
        const allowedHostnames = ['bbsmc.net']

        if (allowedHostnames.includes(url.hostname)) {
          return defaultLinkOpenRenderer(tokens, idx, options, env, self)
        }
      } catch {
        /* empty */
      }
    }

    tokens[idx].attrSet('rel', 'noopener nofollow ugc')

    return defaultLinkOpenRenderer(tokens, idx, options, env, self)
  }

  return md
}

export const renderString = (string: string) => configuredXss.process(md().render(string))
