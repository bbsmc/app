<template>
  <div class="tutorial-page" :class="{ 'tutorial-dark': isDark }">
    <!-- Side nav -->
    <nav class="side-nav">
      <div
        v-for="(sec, i) in sections"
        :key="i"
        class="sn-item"
        :class="{ active: activeSection === i }"
        @click="gotoSec(i)"
      >
        <span class="sn-label">{{ sec.label }}</span>
        <div class="sn-dot"></div>
      </div>
    </nav>

    <!-- Hero -->
    <section id="sec-0" class="hero">
      <div class="hero-eyebrow">汉化补丁覆盖安装</div>
      <h1 class="hero-title">补丁安装<br /><em>全流程</em>教程</h1>
      <p class="hero-lead">
        适用于 PCL2 启动器。从下载汉化包到覆盖安装，包含版本隔离判断与文件冲突处理，全程图解说明。
      </p>

      <div class="callout note intro-callout">
        <div class="ci">&#128218;</div>
        <div>
          寒假到了，你和朋友决定一起玩一款外网整合包。满怀期待地启动游戏，然而进入服务器后看着未经汉化的任务书，脑袋里只剩四个字：<strong
            >"哦豁，完蛋。"</strong
          >
          —— 别担心，跟着这篇教程，轻松搞定汉化。
        </div>
      </div>

      <div class="sc-card intro-img">
        <div class="sc-bar">
          <div class="sc-dot r"></div>
          <div class="sc-dot y"></div>
          <div class="sc-dot g"></div>
          <span class="sc-bar-title">未汉化的游戏界面</span>
        </div>
        <img src="/tutorial/bili-unlocalized.png" alt="未汉化的任务书" loading="lazy" />
      </div>

      <div class="warning-strip">
        <div class="ws-icon">&#9888;</div>
        <div>
          <div class="ws-title">版本必须完全对应</div>
          <div class="ws-body">
            整合包是 <code>1.9.1</code> 就必须下载
            <code>1.9.1</code> 的汉化包，<strong>跨版本使用会导致游戏异常</strong>。缺少对应版本请加
            QQ 群 <strong>1073724937</strong> 提需求。
          </div>
        </div>
      </div>

      <div class="video-wrap">
        <iframe
          src="//player.bilibili.com/player.html?isOutside=true&aid=116131805269869&bvid=BV1PsfhBCEeb&cid=36284727610&p=1"
          scrolling="no"
          border="0"
          frameborder="no"
          framespacing="0"
          allowfullscreen="true"
        ></iframe>
      </div>
    </section>

    <div class="content-wrap">
      <!-- STEP 1 -->
      <div id="sec-1" class="step-block">
        <div class="step-header">
          <div class="step-n">01</div>
          <div>
            <div class="step-chapter">第一步</div>
            <div class="step-title">在 BBSMC 上找到汉化包</div>
          </div>
        </div>
        <div class="step-body">
          <p class="step-desc">
            如果你是从 BBSMC
            上下载的整合包，在整合包下载页面的右侧会显示<strong>「汉化包」链接</strong>，点击即可跳转到对应的汉化包页面。
          </p>

          <div class="sc-card">
            <div class="sc-bar">
              <div class="sc-dot r"></div>
              <div class="sc-dot y"></div>
              <div class="sc-dot g"></div>
              <span class="sc-bar-title">BBSMC — 整合包页面右侧汉化包入口</span>
            </div>
            <img
              src="/tutorial/bili-modpack-link.png"
              alt="BBSMC整合包页面汉化包链接"
              loading="lazy"
            />
          </div>

          <p class="step-desc">
            你也可以直接在 BBSMC
            首页点击<strong>「汉化」</strong>分类，像搜索整合包一样搜索你需要的汉化包。
          </p>

          <div class="sc-card">
            <div class="sc-bar">
              <div class="sc-dot r"></div>
              <div class="sc-dot y"></div>
              <div class="sc-dot g"></div>
              <span class="sc-bar-title">BBSMC — 搜索汉化包</span>
            </div>
            <img src="/tutorial/bili-search-language.png" alt="BBSMC搜索汉化包" loading="lazy" />
          </div>
        </div>
      </div>

      <!-- STEP 2 -->
      <div id="sec-2" class="step-block">
        <div class="step-header">
          <div class="step-n">02</div>
          <div>
            <div class="step-chapter">第二步</div>
            <div class="step-title">下载正确版本的汉化包</div>
          </div>
        </div>
        <div class="step-body">
          <p class="step-desc">
            打开汉化包页面后，点击上方的<strong>「下载」按钮</strong>，跟随指引下载与你整合包<strong>版本号完全一致</strong>的汉化包。下载的文件是
            <code>.zip</code> 压缩包。
          </p>

          <div class="sc-card">
            <div class="sc-bar">
              <div class="sc-dot r"></div>
              <div class="sc-dot y"></div>
              <div class="sc-dot g"></div>
              <span class="sc-bar-title">BBSMC — 汉化包下载界面</span>
            </div>
            <img src="/tutorial/bili-download-page.png" alt="汉化包下载界面" loading="lazy" />
          </div>

          <div class="ver-check">
            <div class="vc-head">版本对照示例</div>
            <div class="vc-row">
              <span class="vc-k">整合包版本</span><span class="vc-v">1.9.1</span>
            </div>
            <div class="vc-row">
              <span class="vc-k">汉化包版本</span><span class="vc-v">1.9.1</span>
            </div>
            <div class="vc-row">
              <span class="vc-k">匹配状态</span
              ><span class="vc-v vc-ok">&#10003; 版本一致，可以使用</span>
            </div>
          </div>

          <div class="callout warn">
            <div class="ci">&#9888;</div>
            <div>
              一定要<strong>再三确定</strong>汉化包的版本和整合包的版本是相对应的。跨版本使用会导致游戏崩溃或异常。
            </div>
          </div>

          <div class="callout note">
            <div class="ci">&#128161;</div>
            <div>
              下载完毕后请确保你的电脑上至少安装了一款解压缩软件（如
              Bandizip、7-Zip）。如果没有安装，请参考本页底部的<strong>「Bandizip 安装教程」</strong
              >。
            </div>
          </div>
        </div>
      </div>

      <!-- STEP 3 -->
      <div id="sec-3" class="step-block">
        <div class="step-header">
          <div class="step-n">03</div>
          <div>
            <div class="step-chapter">第三步</div>
            <div class="step-title">通过 PCL2 打开整合包目录</div>
          </div>
        </div>
        <div class="step-body">
          <p class="step-desc">
            将汉化包内的文件<strong>解压到一个文件夹内</strong>。之后打开 PCL2
            启动器，选中目标整合包，点击<strong>「版本设置」</strong>，进入设置页面后点击<strong>「版本文件夹」</strong>，文件管理器将自动定位到正确的整合包目录。
          </p>

          <div class="sc-card">
            <div class="sc-bar">
              <div class="sc-dot r"></div>
              <div class="sc-dot y"></div>
              <div class="sc-dot g"></div>
              <span class="sc-bar-title">PCL2 — 版本设置 → 版本文件夹</span>
            </div>
            <img src="/tutorial/bili-pcl2-settings.png" alt="PCL2版本设置" loading="lazy" />
          </div>

          <div class="two-col">
            <div class="col-card blue">
              <span class="col-tag">开启了版本隔离</span>
              <h4>独立实例目录</h4>
              <p>
                整合包在 .minecraft/versions/整合包名称 文件夹中，每个整合包拥有独立的 mods、config
              </p>
              <div class="path-chip">
                <small>路径形如</small>…\versions\<span class="hl">整合包名称</span>\
              </div>
            </div>
            <div class="col-card teal">
              <span class="col-tag">未开启版本隔离</span>
              <h4>公共 .minecraft 目录</h4>
              <p>所有版本共用同一套 mods、config（PCL2 自定义路径下）</p>
              <div class="path-chip">
                <small>路径形如</small>PCL自定义路径\<span class="hl">.minecraft</span>\
              </div>
            </div>
          </div>

          <div class="callout ok">
            <div class="ci">&#10004;</div>
            <div>
              点击「版本文件夹」后文件管理器直接打开整合包根目录。<strong>无论是否开启版本隔离，此方法均适用</strong>，不需要手动寻找路径。
            </div>
          </div>

          <ul class="checklist">
            <li>
              <div class="ck">&#10003;</div>
              目录内看到 <code>mods</code> 文件夹
            </li>
            <li>
              <div class="ck">&#10003;</div>
              目录内看到 <code>config</code> 文件夹
            </li>
            <li>
              <div class="ck">&#10003;</div>
              目录内看到 <code>kubejs</code> 或其他整合包专属文件夹
            </li>
          </ul>
        </div>
      </div>

      <!-- STEP 4 -->
      <div id="sec-4" class="step-block">
        <div class="step-header">
          <div class="step-n">04</div>
          <div>
            <div class="step-chapter">第四步</div>
            <div class="step-title">将汉化文件覆盖到目录</div>
          </div>
        </div>
        <div class="step-body">
          <p class="step-desc">
            将刚才解压的汉化文件<strong>全部拖进</strong>整合包所在的文件夹里，覆盖同名文件。弹出「替换或跳过文件」提示时，<strong>必须选择「替换目标中的文件」</strong>。
          </p>

          <div class="sc-card">
            <div class="sc-bar">
              <div class="sc-dot r"></div>
              <div class="sc-dot y"></div>
              <div class="sc-dot g"></div>
              <span class="sc-bar-title"
                >拖拽覆盖文件 — 勾选「为所有当前项目执行此操作」并选择替换</span
              >
            </div>
            <img src="/tutorial/bili-drag-files1.png" alt="覆盖文件并替换" loading="lazy" />
          </div>

          <div class="dialog-outer">
            <div class="dialog-win">
              <div class="dialog-bar">
                <div class="sc-dot r"></div>
                <div class="sc-dot y"></div>
                <div class="sc-dot g"></div>
                <span class="dialog-bar-title">替换或跳过文件</span>
              </div>
              <div class="dialog-body">
                <div class="db-h">目标包含同名文件</div>
                <div class="db-sub">正在将汉化文件复制到整合包目录…</div>
                <div class="d-opt pick">
                  <span class="d-opt-icon">&#10003;</span>
                  <strong>替换目标中的文件(R)</strong>
                  <div class="d-badge">选这个</div>
                </div>
                <div class="d-opt dim"><span class="d-opt-icon">&#9193;</span>跳过这些文件(S)</div>
                <div class="d-opt dim">
                  <span class="d-opt-icon">&#9881;</span>让我决定每个文件(L)
                </div>
              </div>
            </div>
          </div>

          <div class="callout warn">
            <div class="ci">&#9888;</div>
            <div>
              直接将
              <strong>mods、config、kubejs 等文件夹</strong
              >拖到目标目录，<strong>不要</strong>在目标目录里新建一个文件夹再往里放——多一层目录汉化就不会生效。选「跳过」会保留旧文件，汉化也无法生效。
            </div>
          </div>
        </div>
      </div>

      <!-- STEP 5 -->
      <div id="sec-5" class="step-block">
        <div class="step-header">
          <div class="step-n">05</div>
          <div>
            <div class="step-chapter">第五步</div>
            <div class="step-title">启动游戏，享受汉化</div>
          </div>
        </div>
        <div class="step-body">
          <p class="step-desc">
            覆盖完成后重新启动游戏，即可看到汉化效果。如游戏正在运行，需先<strong>完全退出再重启</strong>。
          </p>

          <div class="two-col">
            <div class="sc-card">
              <div class="sc-bar">
                <div class="sc-dot r"></div>
                <div class="sc-dot y"></div>
                <div class="sc-dot g"></div>
                <span class="sc-bar-title">汉化前</span>
              </div>
              <img src="/tutorial/bili-unlocalized.png" alt="汉化前" loading="lazy" />
            </div>
            <div class="sc-card">
              <div class="sc-bar">
                <div class="sc-dot r"></div>
                <div class="sc-dot y"></div>
                <div class="sc-dot g"></div>
                <span class="sc-bar-title">汉化后</span>
              </div>
              <img src="/tutorial/bili-after-cn.png" alt="汉化后（已翻译为中文）" loading="lazy" />
            </div>
          </div>

          <div class="callout ok">
            <div class="ci">&#127918;</div>
            <div>
              现在，你可以畅玩汉化后的整合包了！如果汉化不生效，请检查是否选错了版本或忘记替换文件。
            </div>
          </div>
        </div>
      </div>

      <!-- APPENDIX: Bandizip -->
      <div id="sec-6" class="step-block">
        <div class="step-header">
          <div class="step-n appendix-n">&#128218;</div>
          <div>
            <div class="step-chapter">附录</div>
            <div class="step-title">Bandizip 下载与安装</div>
          </div>
        </div>
        <div class="step-body">
          <p class="step-desc">
            如果你的电脑上没有安装过解压缩软件，可以免费安装 Bandizip。在浏览器中访问
            <a href="https://www.bandisoft.com/bandizip/" target="_blank" class="text-link"
              >https://www.bandisoft.com/bandizip/</a
            >
            进入下载页面。
          </p>

          <div class="two-col">
            <div class="sc-card">
              <div class="sc-bar">
                <div class="sc-dot r"></div>
                <div class="sc-dot y"></div>
                <div class="sc-dot g"></div>
                <span class="sc-bar-title">Bandizip 官网</span>
              </div>
              <img src="/tutorial/bili-bandizip-site.png" alt="Bandizip官网" loading="lazy" />
            </div>
            <div class="sc-card">
              <div class="sc-bar">
                <div class="sc-dot r"></div>
                <div class="sc-dot y"></div>
                <div class="sc-dot g"></div>
                <span class="sc-bar-title">点击免费下载</span>
              </div>
              <img src="/tutorial/bili-bandizip-download.png" alt="下载Bandizip" loading="lazy" />
            </div>
          </div>

          <p class="step-desc">
            下载安装包后打开，选择左边的<strong>「评价版」</strong>（免费），点击<strong>「同意并安装」</strong>，安装结束后点击关闭即可。
          </p>

          <div class="callout ok">
            <div class="ci">&#10004;</div>
            <div>安装完成后，你就可以右键任何 <code>.zip</code> 文件进行解压了。</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Success -->
    <section id="sec-7" class="success-section">
      <div class="success-left">
        <span class="suc-emoji"></span>
        <div class="suc-title">安装完成</div>
        <div class="suc-sub">重启游戏，享受完整中文汉化体验。</div>
      </div>
      <div class="qq-card">
        <h4>缺少版本的汉化包？</h4>
        <p>加入 BBSMC 汉化组 QQ 群提交需求，快速响应。</p>
        <span class="qq-num">1073724937</span>
      </div>
    </section>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { isDarkTheme } from "~/plugins/theme/themes.ts";

const { $theme } = useNuxtApp();
const isDark = computed(() => isDarkTheme($theme?.active));

useHead({
  title: "汉化补丁安装教程 - BBSMC",
  meta: [
    {
      name: "description",
      content:
        "BBSMC汉化补丁安装教程，适用于PCL2启动器，从下载汉化包到覆盖安装，包含版本隔离判断与文件冲突处理，全程图解说明。",
    },
  ],
  link: [
    {
      rel: "preconnect",
      href: "https://fonts.googleapis.com",
    },
    {
      rel: "stylesheet",
      href: "https://fonts.googleapis.com/css2?family=Noto+Serif+SC:wght@700;900&display=swap",
    },
  ],
});

const sections = [
  { label: "概述" },
  { label: "01 找汉化包" },
  { label: "02 下载" },
  { label: "03 找到目录" },
  { label: "04 覆盖安装" },
  { label: "05 启动游戏" },
  { label: "附录" },
  { label: "完成" },
];

const secIds = ["sec-0", "sec-1", "sec-2", "sec-3", "sec-4", "sec-5", "sec-6", "sec-7"];
const activeSection = ref(0);

let stepObserver = null;
let sectionObserver = null;

function gotoSec(i) {
  document.getElementById(secIds[i])?.scrollIntoView({ behavior: "smooth" });
}

onMounted(() => {
  // Step block reveal animation
  const blocks = document.querySelectorAll(".tutorial-page .step-block");
  stepObserver = new IntersectionObserver(
    (entries) => {
      entries.forEach((e) => {
        if (e.isIntersecting) e.target.classList.add("visible");
      });
    },
    { threshold: 0.07 },
  );
  blocks.forEach((b) => {
    stepObserver.observe(b);
    if (b.getBoundingClientRect().top < window.innerHeight * 0.94) {
      b.classList.add("visible");
    }
  });

  // Side nav active tracking
  sectionObserver = new IntersectionObserver(
    (entries) => {
      entries.forEach((e) => {
        if (e.isIntersecting) {
          const i = secIds.indexOf(e.target.id);
          if (i >= 0) activeSection.value = i;
        }
      });
    },
    { rootMargin: "-25% 0px -65% 0px" },
  );
  secIds.forEach((id) => {
    const el = document.getElementById(id);
    if (el) sectionObserver.observe(el);
  });
});

onBeforeUnmount(() => {
  stepObserver?.disconnect();
  sectionObserver?.disconnect();
});
</script>

<style scoped>
/* ── Theme-aware variables ── */
.tutorial-page {
  --t-ink: var(--color-text-dark);
  --t-ink-2: var(--color-text);
  --t-ink-3: var(--color-secondary);
  --t-ink-4: var(--color-text-inactive);
  --t-paper: var(--color-bg);
  --t-raised: var(--color-raised-bg);
  --t-rule: var(--color-divider-dark);
  --t-orange: var(--flame);
  --t-mono: var(--font-mono);

  --t-paper-2: #f0eeec;
  --t-paper-3: #e6e3e0;
  --t-shadow: #d8d4d0;
  --t-orange-bg: #fff3ec;
  --t-orange-bd: #f5c9aa;
  --t-green: #3a6b43;
  --t-green-bg: #edf5ee;
  --t-green-bd: #b2d4b8;
  --t-amber: #a05c00;
  --t-amber-bg: #fff8ec;
  --t-amber-bd: #f5dba0;
  --t-blue-bg: #f4f8ff;
  --t-blue-bd: #b8d0ee;
  --t-blue-accent: #4080c0;
  --t-blue-tag-bg: #d8e8ff;
  --t-blue-tag-color: #205098;
  --t-blue-shadow: #c4d4f0;
  --t-green-shadow: #b8d4bc;
  --t-folder-color: #8a5c00;
  --t-path-overlay: rgba(0, 0, 0, 0.04);
  --t-dot-border: rgba(0, 0, 0, 0.12);

  background: var(--t-paper);
  color: var(--t-ink-2);
  font-family: var(--font-body);
  font-size: 15px;
  line-height: 1.75;
  min-height: 100vh;
  margin: -16px;
  padding: 0;
}

/* ── Dark mode overrides ── */
.tutorial-page.tutorial-dark {
  --t-paper-2: #14171c;
  --t-paper-3: #1e2229;
  --t-shadow: #0a0c10;
  --t-orange-bg: rgba(241, 100, 54, 0.1);
  --t-orange-bd: rgba(241, 100, 54, 0.25);
  --t-green: #4ade80;
  --t-green-bg: rgba(27, 217, 106, 0.1);
  --t-green-bd: rgba(27, 217, 106, 0.2);
  --t-amber: #ffb347;
  --t-amber-bg: rgba(255, 163, 71, 0.1);
  --t-amber-bd: rgba(255, 163, 71, 0.2);
  --t-blue-bg: rgba(79, 156, 255, 0.08);
  --t-blue-bd: rgba(79, 156, 255, 0.2);
  --t-blue-accent: #4f9cff;
  --t-blue-tag-bg: rgba(79, 156, 255, 0.15);
  --t-blue-tag-color: #7bb8ff;
  --t-blue-shadow: rgba(79, 156, 255, 0.08);
  --t-green-shadow: rgba(27, 217, 106, 0.08);
  --t-folder-color: #d4a73a;
  --t-path-overlay: rgba(255, 255, 255, 0.04);
  --t-dot-border: rgba(255, 255, 255, 0.15);
}

.tutorial-page code,
.tutorial-page kbd {
  font-family: var(--t-mono);
  font-size: 0.85em;
  background: var(--t-paper-3);
  border: 1px solid var(--t-rule);
  border-radius: 3px;
  padding: 1px 6px;
  color: var(--t-ink-2);
}

/* HERO */
.hero {
  max-width: 820px;
  margin: 0 auto;
  padding: 72px 48px 56px;
  text-align: center;
}

.hero-eyebrow {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 22px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  color: var(--t-orange);
}

.hero-eyebrow::after {
  content: "";
  flex: 1;
  height: 1px;
  background: var(--t-rule);
}

.hero-title {
  font-family: "Noto Serif SC", serif, var(--font-body);
  font-size: clamp(38px, 6vw, 68px);
  font-weight: 900;
  line-height: 1.06;
  color: var(--t-ink);
  letter-spacing: -0.025em;
  margin-bottom: 22px;
}

.hero-title em {
  font-style: normal;
  color: var(--t-orange);
}

.hero-lead {
  font-size: 16px;
  color: var(--t-ink-3);
  max-width: 520px;
  line-height: 1.72;
  margin: 0 auto 32px;
}

.intro-callout {
  max-width: 620px;
  margin: 0 auto 24px;
  text-align: left;
}

.intro-img {
  max-width: 620px;
  margin: 0 auto 24px;
}

/* VIDEO */
.video-wrap {
  position: relative;
  width: 100%;
  max-width: 620px;
  margin: 32px auto 0;
  padding-bottom: 56.25%;
  height: 0;
  overflow: hidden;
  border-radius: 5px;
  border: 1.5px solid var(--t-rule);
  box-shadow: 3px 4px 0 var(--t-shadow);
}

.video-wrap iframe {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  border: 0;
}

.warning-strip {
  background: var(--t-amber-bg);
  border: 1px solid var(--t-amber-bd);
  border-left: 4px solid var(--t-amber);
  border-radius: 3px;
  padding: 16px 20px;
  display: flex;
  gap: 14px;
  align-items: flex-start;
  max-width: 620px;
  margin: 0 auto;
  text-align: left;
}

.ws-icon {
  font-size: 17px;
  flex-shrink: 0;
  margin-top: 2px;
}
.ws-title {
  font-weight: 700;
  font-size: 13px;
  color: var(--t-amber);
  margin-bottom: 5px;
}
.ws-body {
  font-size: 13px;
  color: var(--t-ink-2);
  line-height: 1.65;
}

/* LAYOUT */
.content-wrap {
  max-width: 820px;
  margin: 0 auto;
  padding: 0 48px 80px;
}

/* STEP BLOCK */
.step-block {
  border-top: 2px solid var(--t-ink);
  padding: 48px 0 0;
  margin-bottom: 52px;
  opacity: 0;
  transform: translateY(18px);
  transition:
    opacity 0.5s ease,
    transform 0.5s ease;
}

.step-block.visible {
  opacity: 1;
  transform: translateY(0);
}

.step-header {
  display: grid;
  grid-template-columns: 60px 1fr;
  gap: 0 20px;
  align-items: baseline;
  margin-bottom: 24px;
}

.step-n {
  font-family: "Noto Serif SC", serif, var(--font-body);
  font-size: 52px;
  font-weight: 900;
  color: var(--t-orange);
  line-height: 1;
  letter-spacing: -0.04em;
  user-select: none;
}

.step-n.appendix-n {
  font-size: 40px;
  text-align: center;
}

.step-chapter {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--t-ink-4);
  margin-bottom: 5px;
}

.step-title {
  font-family: "Noto Serif SC", serif, var(--font-body);
  font-size: 24px;
  font-weight: 700;
  color: var(--t-ink);
  letter-spacing: -0.01em;
  line-height: 1.2;
}

.step-body {
  padding-left: 80px;
}

.step-desc {
  color: var(--t-ink-2);
  font-size: 14.5px;
  line-height: 1.82;
  margin-bottom: 24px;
}

.step-desc strong {
  color: var(--t-ink);
  font-weight: 700;
}

.step-desc .text-link {
  color: var(--t-blue-accent);
  text-decoration: underline;
}

/* SCREENSHOT CARD */
.sc-card {
  background: var(--t-raised);
  border: 1.5px solid var(--t-rule);
  border-radius: 5px;
  overflow: hidden;
  margin-bottom: 20px;
  box-shadow: 3px 4px 0 var(--t-shadow);
  transition:
    transform 0.18s,
    box-shadow 0.18s;
}

.sc-card:hover {
  transform: translate(-1px, -2px);
  box-shadow: 5px 6px 0 var(--t-shadow);
}

.sc-bar {
  background: var(--t-paper-2);
  border-bottom: 1px solid var(--t-rule);
  padding: 8px 14px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.sc-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  border: 1px solid var(--t-dot-border);
  flex-shrink: 0;
}
.sc-dot.r {
  background: #ff5f57;
}
.sc-dot.y {
  background: #febc2e;
}
.sc-dot.g {
  background: #28c840;
}

.sc-bar-title {
  font-family: var(--t-mono);
  font-size: 11px;
  color: var(--t-ink-3);
  flex: 1;
  text-align: center;
}

.sc-card > img {
  width: 100%;
  display: block;
}

/* TWO-COL CARDS */
.two-col {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
  margin-bottom: 20px;
}

.col-card {
  border: 1.5px solid var(--t-rule);
  border-radius: 5px;
  padding: 18px;
  background: var(--t-raised);
  box-shadow: 2px 3px 0 var(--t-shadow);
}

.col-card.blue {
  border-color: var(--t-blue-bd);
  background: var(--t-blue-bg);
  box-shadow: 2px 3px 0 var(--t-blue-shadow);
}
.col-card.teal {
  border-color: var(--t-green-bd);
  background: var(--t-green-bg);
  box-shadow: 2px 3px 0 var(--t-green-shadow);
}

.col-tag {
  display: inline-block;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  padding: 2px 8px;
  border-radius: 2px;
  margin-bottom: 9px;
}

.blue .col-tag {
  background: var(--t-blue-tag-bg);
  color: var(--t-blue-tag-color);
}
.teal .col-tag {
  background: var(--t-green-bd);
  color: var(--t-green);
}

.col-card h4 {
  font-size: 14px;
  font-weight: 700;
  margin-bottom: 6px;
  color: var(--t-ink);
  font-family: "Noto Serif SC", serif, var(--font-body);
}

.col-card p {
  font-size: 12.5px;
  color: var(--t-ink-2);
  line-height: 1.65;
}

.path-chip {
  margin-top: 10px;
  padding: 8px 12px;
  background: var(--t-path-overlay);
  border: 1px solid var(--t-rule);
  border-radius: 3px;
  font-family: var(--t-mono);
  font-size: 11px;
  color: var(--t-ink-2);
  line-height: 1.5;
  word-break: break-all;
}

.path-chip small {
  display: block;
  font-family: var(--font-body);
  font-size: 10px;
  color: var(--t-ink-4);
  margin-bottom: 3px;
}
.path-chip .hl {
  color: var(--t-orange);
  font-weight: 500;
}

/* CHECKLIST */
.checklist {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 20px;
  padding: 0;
}

.checklist li {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13.5px;
  color: var(--t-ink-2);
  padding: 9px 14px;
  background: var(--t-green-bg);
  border: 1px solid var(--t-green-bd);
  border-radius: 3px;
  opacity: 0;
  animation: checkAppear 0.3s ease forwards;
}

.checklist li:nth-child(1) {
  animation-delay: 0.05s;
}
.checklist li:nth-child(2) {
  animation-delay: 0.12s;
}
.checklist li:nth-child(3) {
  animation-delay: 0.19s;
}

.ck {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--t-green);
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

/* CALLOUT */
.callout {
  display: flex;
  gap: 12px;
  padding: 14px 18px;
  border-radius: 3px;
  font-size: 13.5px;
  line-height: 1.72;
  margin-bottom: 20px;
}

.callout.note {
  background: var(--t-blue-bg);
  border: 1px solid var(--t-blue-bd);
  border-left: 3px solid var(--t-blue-accent);
  color: var(--t-ink-2);
}
.callout.warn {
  background: var(--t-orange-bg);
  border: 1px solid var(--t-orange-bd);
  border-left: 3px solid var(--t-orange);
  color: var(--t-ink-2);
}
.callout.ok {
  background: var(--t-green-bg);
  border: 1px solid var(--t-green-bd);
  border-left: 3px solid var(--t-green);
  color: var(--t-ink-2);
}

.callout .ci {
  font-size: 15px;
  flex-shrink: 0;
  margin-top: 2px;
}
.callout strong {
  color: var(--t-ink);
}

/* VERSION CHECK */
.ver-check {
  background: var(--t-raised);
  border: 1.5px solid var(--t-rule);
  border-radius: 5px;
  overflow: hidden;
  margin-bottom: 20px;
  box-shadow: 2px 3px 0 var(--t-shadow);
}

.vc-head {
  background: var(--t-paper-2);
  border-bottom: 1px solid var(--t-rule);
  padding: 9px 18px;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  color: var(--t-ink-3);
}

.vc-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 18px;
  border-bottom: 1px solid var(--t-rule);
  font-size: 13.5px;
}

.vc-row:last-child {
  border-bottom: none;
}
.vc-k {
  color: var(--t-ink-3);
}
.vc-v {
  font-family: var(--t-mono);
  font-size: 12.5px;
  font-weight: 500;
}
.vc-ok {
  color: var(--t-green);
  font-weight: 700;
}

/* DIALOG */
.dialog-outer {
  display: flex;
  justify-content: center;
  margin-bottom: 20px;
}

.dialog-win {
  background: var(--t-raised);
  border: 1.5px solid var(--t-rule);
  border-radius: 5px;
  overflow: hidden;
  width: 370px;
  max-width: 100%;
  box-shadow: 4px 6px 0 var(--t-shadow);
  animation: dialogIn 0.32s cubic-bezier(0.22, 1, 0.36, 1) both;
}

.dialog-bar {
  background: var(--t-paper-2);
  border-bottom: 1px solid var(--t-rule);
  padding: 9px 14px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.dialog-bar-title {
  font-family: var(--t-mono);
  font-size: 11px;
  color: var(--t-ink-3);
  flex: 1;
  text-align: center;
}

.dialog-body {
  padding: 20px;
}
.db-h {
  font-weight: 700;
  font-size: 14px;
  margin-bottom: 4px;
}
.db-sub {
  font-size: 12px;
  color: var(--t-ink-3);
  margin-bottom: 16px;
}

.d-opt {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 14px;
  border-radius: 3px;
  border: 1px solid var(--t-rule);
  margin-bottom: 8px;
  font-size: 13px;
  background: var(--t-raised);
  color: var(--t-ink);
}

.d-opt.pick {
  background: var(--t-orange-bg);
  border-color: var(--t-orange);
  color: var(--t-orange);
  font-weight: 700;
  animation: pickPulse 2s 1s ease infinite;
}

.d-opt.dim {
  color: var(--t-ink-3);
}
.d-opt-icon {
  font-size: 14px;
  flex-shrink: 0;
}
.d-badge {
  margin-left: auto;
  font-size: 10px;
  font-weight: 700;
  background: var(--t-orange);
  color: #fff;
  padding: 2px 7px;
  border-radius: 2px;
}

/* SUCCESS */
.success-section {
  border-top: 2px solid var(--t-ink);
  max-width: 820px;
  margin: 0 auto;
  padding: 64px 48px 80px;
  display: flex;
  gap: 48px;
  align-items: flex-start;
}

.suc-emoji {
  font-size: 52px;
  display: block;
  margin-bottom: 16px;
  animation: successIn 0.5s 0.2s cubic-bezier(0.34, 1.56, 0.64, 1) both;
}
.suc-title {
  font-family: "Noto Serif SC", serif, var(--font-body);
  font-size: 36px;
  font-weight: 900;
  color: var(--t-green);
  letter-spacing: -0.025em;
  margin-bottom: 8px;
}
.suc-sub {
  font-size: 14px;
  color: var(--t-ink-3);
}

.qq-card {
  background: var(--t-paper-2);
  border: 1.5px solid var(--t-rule);
  border-radius: 5px;
  padding: 22px;
  min-width: 210px;
  box-shadow: 3px 4px 0 var(--t-shadow);
  flex-shrink: 0;
}

.qq-card h4 {
  font-size: 13px;
  font-weight: 700;
  margin-bottom: 6px;
  color: var(--t-ink);
}
.qq-card p {
  font-size: 12px;
  color: var(--t-ink-3);
  line-height: 1.6;
  margin-bottom: 12px;
}
.qq-num {
  font-family: var(--t-mono);
  font-size: 19px;
  font-weight: 500;
  color: var(--t-orange);
}

/* SIDE NAV */
.side-nav {
  position: fixed;
  right: 26px;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 200;
}

.sn-item {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  justify-content: flex-end;
  padding: 3px 0;
}

.sn-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--t-paper-3);
  border: 1.5px solid var(--t-rule);
  transition: all 0.18s;
  flex-shrink: 0;
}

.sn-item.active .sn-dot {
  background: var(--t-orange);
  border-color: var(--t-orange);
  width: 9px;
  height: 9px;
}
.sn-item:hover .sn-dot {
  border-color: var(--t-orange);
}

.sn-label {
  font-size: 10px;
  color: var(--t-ink-3);
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.15s;
  white-space: nowrap;
  font-weight: 500;
}

.sn-item:hover .sn-label {
  opacity: 1;
}

/* RESPONSIVE */
@media (max-width: 900px) {
  .side-nav {
    display: none;
  }
}
@media (max-width: 640px) {
  .hero,
  .content-wrap,
  .success-section {
    padding-left: 20px;
    padding-right: 20px;
  }
  .step-header {
    grid-template-columns: 48px 1fr;
    gap: 0 14px;
  }
  .step-body {
    padding-left: 62px;
  }
  .two-col {
    grid-template-columns: 1fr;
  }
  .success-section {
    flex-direction: column;
    gap: 32px;
  }
}

/* ANIMATIONS */
@keyframes dialogIn {
  from {
    opacity: 0;
    transform: scale(0.93) translateY(8px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@keyframes pickPulse {
  0%,
  100% {
    box-shadow: none;
  }
  50% {
    box-shadow: 0 0 0 3px var(--t-orange-bd);
  }
}

@keyframes checkAppear {
  from {
    opacity: 0;
    transform: translateX(-5px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

@keyframes successIn {
  from {
    opacity: 0;
    transform: scale(0);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
</style>
