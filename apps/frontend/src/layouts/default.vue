<template>
  <div ref="main_page" class="layout" :class="{ 'expanded-mobile-nav': isBrowseMenuOpen }">
    <!--    邮箱验证提示-->
    <div
      v-if="auth?.user && !auth.user.email_verified && route.path !== '/auth/verify-email'"
      class="email-nag"
    >
      <template v-if="auth.user?.email">
        <span>{{ formatMessage(verifyEmailBannerMessages.title) }}</span>
        <button class="btn" @click="resendVerifyEmail">
          {{ formatMessage(verifyEmailBannerMessages.action) }}
        </button>
      </template>
      <template v-else>
        <span>{{ formatMessage(addEmailBannerMessages.title) }}</span>
        <nuxt-link class="btn" to="/settings/account">
          <SettingsIcon aria-hidden="true" />
          {{ formatMessage(addEmailBannerMessages.action) }}
        </nuxt-link>
      </template>
    </div>

    <!--    封禁提示-->
    <div v-if="auth?.user?.active_bans && auth.user.active_bans.length > 0" class="ban-nag">
      <ShieldIcon aria-hidden="true" class="ban-icon" />
      <span>
        您的账户当前存在封禁限制，部分功能可能受到限制。
        <template v-if="auth.user.active_bans.some((b) => b.appeal)">
          您已提交申诉，请等待审核。
        </template>
      </span>
      <nuxt-link class="btn" to="/settings/account"> 查看详情 </nuxt-link>
    </div>

    <!--    头部 v10 Flame Theme -->
    <header class="site-header desktop-only">
      <div class="header-container">
        <!-- Logo -->
        <NuxtLink to="/" class="header-logo" aria-label="BBSMC home page">
          <BrandTextLogo aria-hidden="true" />
        </NuxtLink>

        <!-- Navigation -->
        <nav class="header-nav">
          <NuxtLink
            to="/mods"
            class="nav-link"
            :class="{ active: route.name === 'search-mods' || route.path.startsWith('/mod/') }"
          >
            <BoxIcon aria-hidden="true" />
            <span>模组</span>
          </NuxtLink>
          <NuxtLink
            to="/modpacks"
            class="nav-link"
            :class="{
              active: route.name === 'search-modpacks' || route.path.startsWith('/modpack/'),
            }"
          >
            <PackageOpenIcon aria-hidden="true" />
            <span>整合包</span>
          </NuxtLink>
          <NuxtLink
            to="/shaders"
            class="nav-link"
            :class="{
              active: route.name === 'search-shaders' || route.path.startsWith('/shader/'),
            }"
          >
            <GlassesIcon aria-hidden="true" />
            <span>光影</span>
          </NuxtLink>
          <NuxtLink
            to="/resourcepacks"
            class="nav-link"
            :class="{
              active:
                route.name === 'search-resourcepacks' || route.path.startsWith('/resourcepack/'),
            }"
          >
            <PaintBrushIcon aria-hidden="true" />
            <span>资源包</span>
          </NuxtLink>
          <NuxtLink
            to="/softwares"
            class="nav-link"
            :class="{
              active: route.name === 'search-softwares' || route.path.startsWith('/software/'),
            }"
          >
            <GridIcon aria-hidden="true" />
            <span>软件</span>
          </NuxtLink>
          <NuxtLink
            to="/languages"
            class="nav-link"
            :class="{
              active: route.name === 'search-languages' || route.path.startsWith('/language/'),
            }"
          >
            <LanguagesIcon aria-hidden="true" />
            <span>汉化</span>
          </NuxtLink>
          <NuxtLink
            to="/plugins"
            class="nav-link"
            :class="{
              active: route.name === 'search-plugins' || route.path.startsWith('/plugin/'),
            }"
          >
            <PlugIcon aria-hidden="true" />
            <span>插件</span>
          </NuxtLink>
          <NuxtLink
            to="/datapacks"
            class="nav-link"
            :class="{
              active: route.name === 'search-datapacks' || route.path.startsWith('/datapack/'),
            }"
          >
            <BracesIcon aria-hidden="true" />
            <span>数据包</span>
          </NuxtLink>

          <!-- 论坛模块暂时隐藏
          <NuxtLink
            to="/forums/chat"
            class="nav-link"
            :class="{ active: route.path.startsWith('/forums/') || route.path.startsWith('/d/') }"
          >
            <MessageIcon aria-hidden="true" />
            <span>论坛</span>
          </NuxtLink>
          -->
        </nav>

        <!-- Actions -->
        <div class="header-actions">
          <!-- Create Menu -->
          <OverflowMenu
            v-if="auth?.user"
            class="action-btn create-btn"
            position="bottom"
            direction="left"
            aria-label="Create new..."
            :options="[
              {
                id: 'new-project',
                action: (event) => $refs.modal_creation.show(event),
              },
              {
                id: 'new-collection',
                action: (event) => $refs.modal_collection_creation.show(event),
              },
              { divider: true },
              {
                id: 'new-organization',
                action: (event) => $refs.modal_organization_creation.show(event),
              },
            ]"
          >
            <PlusIcon aria-hidden="true" />
            <template #new-project> <BoxIcon aria-hidden="true" /> 创建资源 </template>
            <template #new-collection> <CollectionIcon aria-hidden="true" /> 创建收藏夹 </template>
            <template #new-organization>
              <OrganizationIcon aria-hidden="true" /> 创建团队
            </template>
          </OverflowMenu>

          <!-- Notifications -->
          <NuxtLink
            v-if="auth?.user"
            to="/dashboard/notifications"
            class="action-btn notification-btn"
          >
            <BellIcon aria-hidden="true" />
            <span v-if="unreadNotifications !== 0" class="notification-dot"></span>
          </NuxtLink>

          <!-- User Menu -->
          <OverflowMenu v-if="auth?.user" class="user-menu" :options="userMenuOptions">
            <Avatar :src="auth.user?.avatar_url" aria-hidden="true" circle class="user-avatar" />
            <template #profile> <UserIcon aria-hidden="true" /> 个人资料 </template>
            <template #notifications> <BellIcon aria-hidden="true" /> 通知 </template>
            <template #saved> <BookmarkIcon aria-hidden="true" /> 收藏夹 </template>
            <template #plus> <ArrowBigUpDashIcon aria-hidden="true" /> 升级到 BBSMC+ </template>
            <template #settings> <SettingsIcon aria-hidden="true" /> 设置 </template>
            <template #flags> <ReportIcon aria-hidden="true" /> 标签 </template>
            <template #projects> <BoxIcon aria-hidden="true" /> 我的资源 </template>
            <template #organizations> <OrganizationIcon aria-hidden="true" /> 团队 </template>
            <template #revenue> <CurrencyIcon aria-hidden="true" /> 收入 </template>
            <template #analytics> <ChartIcon aria-hidden="true" /> 统计 </template>
            <template #moderation> <ModerationIcon aria-hidden="true" /> 管理 </template>
            <template #sign-out> <LogOutIcon aria-hidden="true" /> 登出 </template>
          </OverflowMenu>

          <!-- Login Button -->
          <NuxtLink v-else to="/auth/sign-in" class="login-btn">
            <LogInIcon aria-hidden="true" />
            <span>登录</span>
          </NuxtLink>
        </div>
      </div>
    </header>

    <header class="mobile-navigation mobile-only">
      <div
        class="nav-menu nav-menu-browse"
        :class="{ expanded: isBrowseMenuOpen }"
        @focusin="isBrowseMenuOpen = true"
        @focusout="isBrowseMenuOpen = false"
      >
        <div class="links cascade-links">
          <NuxtLink
            v-for="navRoute in navRoutes"
            :key="navRoute.href"
            :to="navRoute.href"
            class="iconified-button"
          >
            {{ navRoute.label }}
          </NuxtLink>
        </div>
      </div>
      <div
        class="nav-menu nav-menu-mobile"
        :class="{ expanded: isMobileMenuOpen }"
        @focusin="isMobileMenuOpen = true"
        @focusout="isMobileMenuOpen = false"
      >
        <div class="account-container">
          <NuxtLink
            v-if="auth?.user"
            :to="`/user/${auth.user?.username}`"
            class="iconified-button account-button"
          >
            <Avatar
              :src="auth.user?.avatar_url"
              class="user-icon"
              :alt="formatMessage(messages.yourAvatarAlt)"
              aria-hidden="true"
              circle
            />
            <div class="account-text">
              <div>@{{ auth.user?.username }}</div>
              <div>{{ formatMessage(commonMessages.visitYourProfile) }}</div>
            </div>
          </NuxtLink>
          <nuxt-link v-else class="iconified-button brand-button" to="/auth/sign-in">
            <LogInIcon aria-hidden="true" /> {{ formatMessage(commonMessages.signInButton) }}
          </nuxt-link>
        </div>
        <div class="links">
          <template v-if="auth?.user">
            <button class="iconified-button danger-button" @click="logoutUser()">
              <LogOutIcon aria-hidden="true" />
              {{ formatMessage(commonMessages.signOutButton) }}
            </button>
            <button class="iconified-button" @click="$refs.modal_creation.show()">
              <PlusIcon aria-hidden="true" />
              {{ formatMessage(commonMessages.createAProjectButton) }}
            </button>
            <NuxtLink class="iconified-button" to="/dashboard/collections">
              <LibraryIcon class="icon" />
              {{ formatMessage(commonMessages.collectionsLabel) }}
            </NuxtLink>
            <!-- <NuxtLink class="iconified-button" to="/servers/manage">
              <ServerIcon class="icon" />
              {{ formatMessage(commonMessages.serversLabel) }}
            </NuxtLink> -->
            <NuxtLink
              v-if="auth.user?.role === 'moderator' || auth.user?.role === 'admin'"
              class="iconified-button"
              to="/moderation"
            >
              <ModerationIcon aria-hidden="true" />
              {{ formatMessage(commonMessages.moderationLabel) }}
            </NuxtLink>
            <NuxtLink v-if="flags.developerMode" class="iconified-button" to="/flags">
              <ReportIcon aria-hidden="true" />
              功能标签
            </NuxtLink>
          </template>
          <NuxtLink class="iconified-button" to="/settings">
            <SettingsIcon aria-hidden="true" />
            {{ formatMessage(commonMessages.settingsLabel) }}
          </NuxtLink>
          <button class="iconified-button" @click="changeTheme">
            <MoonIcon v-if="$theme.active === 'light'" class="icon" />
            <SunIcon v-else class="icon" />
            <span class="dropdown-item__text">
              {{ formatMessage(messages.changeTheme) }}
            </span>
          </button>
        </div>
      </div>
      <div class="mobile-navbar" :class="{ expanded: isBrowseMenuOpen || isMobileMenuOpen }">
        <NuxtLink
          to="/"
          class="tab button-animation"
          :title="formatMessage(navMenuMessages.home)"
          aria-label="Home"
        >
          <HomeIcon aria-hidden="true" />
        </NuxtLink>
        <button
          class="tab button-animation"
          :class="{ 'router-link-exact-active': isBrowseMenuOpen }"
          :title="formatMessage(navMenuMessages.search)"
          aria-label="Search"
          @click="toggleBrowseMenu()"
        >
          <template v-if="auth?.user">
            <SearchIcon aria-hidden="true" />
          </template>
          <template v-else>
            <SearchIcon aria-hidden="true" class="smaller" />
            {{ formatMessage(navMenuMessages.search) }}
          </template>
        </button>
        <template v-if="auth?.user">
          <NuxtLink
            to="/dashboard/notifications"
            class="tab button-animation"
            aria-label="Notifications"
            :class="{
              'no-active': isMobileMenuOpen || isBrowseMenuOpen,
            }"
            :title="formatMessage(commonMessages.notificationsLabel)"
            @click="
              () => {
                isMobileMenuOpen = false;
                isBrowseMenuOpen = false;
              }
            "
          >
            <NotificationIcon aria-hidden="true" />
          </NuxtLink>
          <NuxtLink
            to="/dashboard"
            class="tab button-animation"
            aria-label="Dashboard"
            :title="formatMessage(commonMessages.dashboardLabel)"
          >
            <ChartIcon aria-hidden="true" />
          </NuxtLink>
        </template>
        <button
          class="tab button-animation"
          :title="formatMessage(messages.toggleMenu)"
          :aria-label="isMobileMenuOpen ? 'Close menu' : 'Open menu'"
          @click="toggleMobileMenu()"
        >
          <template v-if="!auth?.user">
            <HamburgerIcon v-if="!isMobileMenuOpen" aria-hidden="true" />
            <CrossIcon v-else aria-hidden="true" />
          </template>
          <template v-else>
            <Avatar
              :src="auth.user?.avatar_url"
              class="user-icon"
              :class="{ expanded: isMobileMenuOpen }"
              :alt="formatMessage(messages.yourAvatarAlt)"
              aria-hidden="true"
              circle
            />
          </template>
        </button>
      </div>
    </header>
    <main>
      <ModalCreation v-if="auth?.user" ref="modal_creation" />
      <CollectionCreateModal ref="modal_collection_creation" />
      <OrganizationCreateModal ref="modal_organization_creation" />
      <slot id="main" />
    </main>
    <footer class="site-footer">
      <div class="footer-container">
        <!-- Logo and Branding -->
        <div class="footer-brand">
          <NuxtLink to="/" class="footer-logo">
            <BrandTextLogo aria-hidden="true" class="h-8 w-auto" />
          </NuxtLink>
          <p class="footer-tagline">中国最活跃的 Minecraft 中文资源社区</p>
          <a href="javascript:void(0)" class="footer-qq-group" @click="copyQQGroup">
            QQ 群：1078515449
          </a>
          <div class="footer-social">
            <a
              href="https://github.com/bbsmc/app"
              target="_blank"
              rel="noopener"
              class="social-link"
            >
              <svg viewBox="0 0 24 24" fill="currentColor" class="social-icon">
                <path
                  d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"
                />
              </svg>
            </a>
            <a
              href="https://space.bilibili.com/291010701"
              target="_blank"
              rel="noopener"
              class="social-link"
            >
              <svg viewBox="0 0 24 24" fill="currentColor" class="social-icon">
                <path
                  d="M17.813 4.653h.854c1.51.054 2.769.578 3.773 1.574 1.004.995 1.524 2.249 1.56 3.76v7.36c-.036 1.51-.556 2.769-1.56 3.773s-2.262 1.524-3.773 1.56H5.333c-1.51-.036-2.769-.556-3.773-1.56S.036 18.858 0 17.347v-7.36c.036-1.511.556-2.765 1.56-3.76 1.004-.996 2.262-1.52 3.773-1.574h.774l-1.174-1.12a1.234 1.234 0 0 1-.373-.906c0-.356.124-.658.373-.907l.027-.027c.267-.249.573-.373.92-.373.347 0 .653.124.92.373L9.653 4.44c.071.071.134.142.187.213h4.267a.836.836 0 0 1 .16-.213l2.853-2.747c.267-.249.573-.373.92-.373.347 0 .662.151.929.4.267.249.391.551.391.907 0 .355-.124.657-.373.906zM5.333 7.24c-.746.018-1.373.276-1.88.773-.506.498-.769 1.13-.786 1.894v7.52c.017.764.28 1.395.786 1.893.507.498 1.134.756 1.88.773h13.334c.746-.017 1.373-.275 1.88-.773.506-.498.769-1.129.786-1.893v-7.52c-.017-.765-.28-1.396-.786-1.894-.507-.497-1.134-.755-1.88-.773zM8 11.107c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c0-.373.129-.689.386-.947.258-.257.574-.386.947-.386zm8 0c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c.017-.391.15-.711.4-.96.249-.249.56-.373.933-.373z"
                />
              </svg>
            </a>
          </div>
        </div>

        <!-- Quick Links -->
        <div class="footer-links">
          <div class="footer-column">
            <h4 class="footer-column-title">资源</h4>
            <NuxtLink to="/mods" class="footer-link">模组</NuxtLink>
            <NuxtLink to="/modpacks" class="footer-link">整合包</NuxtLink>
            <NuxtLink to="/shaders" class="footer-link">光影</NuxtLink>
            <NuxtLink to="/resourcepacks" class="footer-link">资源包</NuxtLink>
          </div>
          <div class="footer-column">
            <h4 class="footer-column-title">社区</h4>
            <NuxtLink to="/languages" class="footer-link">汉化</NuxtLink>
            <NuxtLink to="/softwares" class="footer-link">软件</NuxtLink>
            <NuxtLink to="/plugins" class="footer-link">插件</NuxtLink>
          </div>
          <div class="footer-column">
            <h4 class="footer-column-title">帮助</h4>
            <NuxtLink to="/legal/terms" class="footer-link">服务条款</NuxtLink>
            <NuxtLink to="/legal/privacy" class="footer-link">隐私政策</NuxtLink>
            <NuxtLink to="/legal/rules" class="footer-link">社区规则</NuxtLink>
            <a
              href="https://github.com/bbsmc/app"
              target="_blank"
              rel="noopener"
              class="footer-link"
              >开源代码</a
            >
          </div>
        </div>

        <!-- Theme Toggle -->
        <div class="footer-actions">
          <button class="theme-toggle" @click="changeTheme">
            <MoonIcon v-if="$theme.active === 'light'" aria-hidden="true" />
            <SunIcon v-else aria-hidden="true" />
            <span>{{ formatMessage(messages.changeTheme) }}</span>
          </button>
          <NuxtLink to="/settings" class="settings-link">
            <SettingsIcon aria-hidden="true" />
            <span>设置</span>
          </NuxtLink>
        </div>
      </div>

      <!-- Bottom Bar -->
      <div class="footer-bottom">
        <div class="footer-bottom-content">
          <p class="footer-disclaimer">
            "Minecraft"以及"我的世界"为美国微软公司的商标，本站与微软公司没有从属关系。 本站与
            Modrinth 无从属关系，网站遵循 LGPL 协议开源。
          </p>
          <p class="footer-copyright">
            © 2019-2026 青岛柒兮网络科技有限公司 |
            <a href="https://beian.miit.gov.cn" target="_blank" rel="noopener">鲁B2-20210590</a> |
            <a href="https://beian.miit.gov.cn" target="_blank" rel="noopener"
              >鲁ICP备2021009459号-12</a
            >
            |
            <a
              href="https://beian.mps.gov.cn/#/query/webSearch?code=37021002001586"
              target="_blank"
              rel="noopener"
              class="police-beian"
            >
              <img src="~/assets/images/beian-police.png" alt="公安备案" />
              鲁公网安备37021002001586号
            </a>
          </p>
        </div>
      </div>
    </footer>
  </div>
</template>
<script setup>
import {
  ArrowBigUpDashIcon,
  BookmarkIcon,
  LogInIcon,
  LibraryIcon,
  ReportIcon,
  HamburgerIcon,
  SearchIcon,
  BellIcon,
  SettingsIcon,
  HomeIcon,
  PlugIcon,
  PlusIcon,
  LogOutIcon,
  ChartIcon,
  BoxIcon,
  CollectionIcon,
  OrganizationIcon,
  UserIcon,
  CurrencyIcon,
  BracesIcon,
  GlassesIcon,
  PaintBrushIcon,
  PackageOpenIcon,
  GridIcon,
  LanguagesIcon,
  ShieldIcon,
  MoonIcon,
  SunIcon,
} from "@modrinth/assets";
import { OverflowMenu, Avatar } from "@modrinth/ui";

import { provide } from "vue";
import CrossIcon from "assets/images/utils/x.svg";
import NotificationIcon from "assets/images/sidebar/notifications.svg";
import ModerationIcon from "assets/images/sidebar/admin.svg";
import ModalCreation from "~/components/ui/ModalCreation.vue";
import { getProjectTypeMessage } from "~/utils/i18n-project-type.ts";
import { commonMessages } from "~/utils/common-messages.ts";
import CollectionCreateModal from "~/components/ui/CollectionCreateModal.vue";
import OrganizationCreateModal from "~/components/ui/OrganizationCreateModal.vue";
import { addNotification } from "~/composables/notifs.js";

const { formatMessage } = useVIntl();

const auth = await useAuth();
const unreadNotifications = ref(0);

// 复制 QQ 群号
const copyQQGroup = () => {
  navigator.clipboard.writeText("1078515449").then(() => {
    addNotification({
      group: "main",
      title: "已复制",
      text: "QQ 群号 1078515449 已复制到剪贴板",
      type: "success",
    });
  });
};

const flags = useFeatureFlags();

const config = useRuntimeConfig();
const route = useNativeRoute();
const link = config.public.siteUrl + route.path.replace(/\/+$/, "");

const verifyEmailBannerMessages = defineMessages({
  title: {
    id: "layout.banner.verify-email.title",
    defaultMessage: "为确保安全，请验证您的邮箱地址。",
  },
  action: {
    id: "layout.banner.verify-email.action",
    defaultMessage: "重新发送验证邮件",
  },
});

const addEmailBannerMessages = defineMessages({
  title: {
    id: "layout.banner.add-email.title",
    defaultMessage: "为确保安全，请绑定您的邮箱地址。",
  },
  action: {
    id: "layout.banner.add-email.button",
    defaultMessage: "前往账户设置",
  },
});

const navMenuMessages = defineMessages({
  home: {
    id: "layout.nav.home",
    defaultMessage: "首页",
  },
  search: {
    id: "layout.nav.search",
    defaultMessage: "搜索",
  },
});

const messages = defineMessages({
  toggleMenu: {
    id: "layout.menu-toggle.action",
    defaultMessage: "切换菜单",
  },
  yourAvatarAlt: {
    id: "layout.avatar.alt",
    defaultMessage: "您的头像",
  },
  getModrinthApp: {
    id: "layout.action.get-modrinth-app",
    defaultMessage: "获取 BBSMC 客户端",
  },
  changeTheme: {
    id: "layout.action.change-theme",
    defaultMessage: "切换主题",
  },
});
defineMessages({
  openSource: {
    id: "layout.footer.open-source",
    defaultMessage: "BBSMC基于Modrinth的开源网站程序修改 <github-link>Github</github-link>.",
  },
  companyTitle: {
    id: "layout.footer.company.title",
    defaultMessage: "关于",
  },
  terms: {
    id: "layout.footer.company.terms",
    defaultMessage: "条款",
  },
  privacy: {
    id: "layout.footer.company.privacy",
    defaultMessage: "隐私",
  },
  rules: {
    id: "layout.footer.company.rules",
    defaultMessage: "规则",
  },
  careers: {
    id: "layout.footer.company.careers",
    defaultMessage: "招聘",
  },
  resourcesTitle: {
    id: "layout.footer.resources.title",
    defaultMessage: "资源",
  },
  support: {
    id: "layout.footer.resources.support",
    defaultMessage: "支持",
  },
  blog: {
    id: "layout.footer.resources.blog",
    defaultMessage: "博客",
  },
  docs: {
    id: "layout.footer.resources.docs",
    defaultMessage: "文档",
  },
  status: {
    id: "layout.footer.resources.status",
    defaultMessage: "状态",
  },
  interactTitle: {
    id: "layout.footer.interact.title",
    defaultMessage: "互动",
  },
  legalDisclaimer: {
    id: "layout.footer.legal-disclaimer",
    defaultMessage: "非官方 MINECRAFT 服务。未获得 MOJANG 或 MICROSOFT 批准或与其相关。",
  },
});
useHead({
  link: [
    {
      rel: "canonical",
      href: link,
    },
  ],
});
useSeoMeta({
  title: "BBSMC - 我的世界资源社区论坛",
  description: () =>
    formatMessage({
      id: "layout.meta.description",
      defaultMessage:
        "在 BBSMC 上下载 Minecraft 模组、插件、数据包、光影、资源包和整合包 " +
        "使用现代、易于使用的界面和 API 在 BBSMC 上发现和发布项目。",
    }),
  publisher: "BBSMC",
  themeColor: "#1bd96a",
  colorScheme: "dark light",

  // OpenGraph
  ogTitle: "BBSMC - 我的世界资源社区论坛",
  ogSiteName: "BBSMC - 我的世界资源社区论坛",
  ogDescription: () =>
    formatMessage({
      id: "layout.meta.og-description",
      defaultMessage:
        "以Minecraft我的世界内容为主的中文论坛。面向我的世界玩家、服主、创作者，提供简洁好用的交流讨论和资源分享平台。你可以在这里找到单机游戏、服务器扩展乃至开发辅助种种资源。并且相较于其他论坛，我们外观精致，UI简洁易用，拥有良好的社区氛围，为你带来非同一般的社区体验",
    }),
  ogType: "website",
  ogImage: "https://cdn.bbsmc.net/raw/bbsmc-logo.png",
  ogUrl: link,

  // Twitter
  twitterCard: "summary_large_image",
  twitterSite: "@bbsmc_net",
});

const isMobileMenuOpen = ref(false);
const isBrowseMenuOpen = ref(false);
const navRoutes = computed(() => [
  {
    id: "mods",
    label: formatMessage(getProjectTypeMessage("mod", true)),
    href: "/mods",
  },
  {
    label: formatMessage(getProjectTypeMessage("plugin", true)),
    href: "/plugins",
  },
  {
    label: formatMessage(getProjectTypeMessage("datapack", true)),
    href: "/datapacks",
  },
  {
    label: formatMessage(getProjectTypeMessage("shader", true)),
    href: "/shaders",
  },
  {
    label: formatMessage(getProjectTypeMessage("resourcepack", true)),
    href: "/resourcepacks",
  },
  {
    label: formatMessage(getProjectTypeMessage("modpack", true)),
    href: "/modpacks",
  },
  {
    label: formatMessage(getProjectTypeMessage("software", true)),
    href: "/softwares",
  },
  {
    label: formatMessage(getProjectTypeMessage("language", true)),
    href: "/languages",
  },
]);

const userMenuOptions = computed(() => {
  if (!auth.value || !auth.value.user) {
    return [];
  }

  let options = [
    {
      id: "profile",
      link: `/user/${auth.value.user.username}`,
    },

    {
      id: "notifications",
      link: "/dashboard/notifications",
    },
    {
      id: "saved",
      link: "/dashboard/collections",
    },
    // {
    //   id: "servers",
    //   link: "/servers/manage",
    // },
    {
      id: "flags",
      link: "/flags",
      shown: flags.value.developerMode,
    },
    {
      id: "settings",
      link: "/settings",
    },
  ];

  // TODO: Only show if user has projects
  options = [
    ...options,
    {
      divider: true,
    },
    {
      id: "projects",
      link: "/dashboard/projects",
    },
    {
      id: "organizations",
      link: "/dashboard/organizations",
    },
    // {
    //   id: "revenue",
    //   link: "/dashboard/revenue",
    // },
    {
      id: "analytics",
      link: "/dashboard/analytics",
    },
  ];

  if (
    auth.value &&
    auth.value.user &&
    (auth.value.user.role === "moderator" || auth.value.user.role === "admin")
  ) {
    options = [
      ...options,
      {
        divider: true,
      },
      {
        id: "moderation",
        color: "orange",
        link: "/moderation",
      },
    ];
  }

  options = [
    ...options,
    {
      divider: true,
    },
    {
      id: "sign-out",
      color: "danger",
      action: () => logoutUser(),
      hoverFilled: true,
    },
  ];
  return options;
});

onMounted(() => {
  if (window && import.meta.client) {
    window.history.scrollRestoration = "auto";
  }

  runAnalytics();
});

watch(
  () => route.path,
  () => {
    isMobileMenuOpen.value = false;
    isBrowseMenuOpen.value = false;

    if (import.meta.client) {
      document.body.style.overflowY = "scroll";
      document.body.setAttribute("tabindex", "-1");
      document.body.removeAttribute("tabindex");
    }

    updateCurrentDate();
    runAnalytics();
  },
);

provide("fetchNotifications", fetchNotifications);

async function fetchNotifications() {
  if (auth.value && auth.value.user) {
    let count = 0;
    const notifications = await useBaseFetch(`user/${auth.value.user.id}/notifications`);
    notifications.forEach((notification) => {
      if (!notification.read) {
        count++;
      }
    });
    unreadNotifications.value = count;
  }
}

// 调用异步函数
fetchNotifications();

async function logoutUser() {
  await logout();
}

function runAnalytics() {
  const config = useRuntimeConfig();
  const replacedUrl = config.public.apiBaseUrl.replace("v2/", "");

  try {
    setTimeout(() => {
      $fetch(`${replacedUrl}analytics/view`, {
        method: "POST",
        body: {
          url: window.location.href,
        },
        headers: {
          Authorization: auth.value.token,
        },
      })
        .then(() => {})
        .catch(() => {});
    });
  } catch (e) {
    console.error(`Sending analytics failed (CORS error? If so, ignore)`, e);
  }
}
function toggleMobileMenu() {
  isMobileMenuOpen.value = !isMobileMenuOpen.value;
  if (isMobileMenuOpen.value) {
    isBrowseMenuOpen.value = false;
  }
}
function toggleBrowseMenu() {
  isBrowseMenuOpen.value = !isBrowseMenuOpen.value;

  if (isBrowseMenuOpen.value) {
    isMobileMenuOpen.value = false;
  }
}
const { cycle: changeTheme } = useTheme();
</script>

<style lang="scss">
@import "~/assets/styles/global.scss";
// @import '@modrinth/assets';

.layout {
  min-height: 100vh;
  background-color: var(--color-bg);
  display: flex;
  flex-direction: column;

  @media screen and (min-width: 1024px) {
    min-height: calc(100vh - var(--spacing-card-bg));
  }

  @media screen and (max-width: 750px) {
    margin-bottom: calc(var(--size-mobile-navbar-height) + 2rem);
  }

  main {
    grid-area: main;
    flex: 1 0 auto;
    padding-top: 24px;
  }

  .site-footer {
    flex-shrink: 0;
  }

  // ==========================================
  // v10 Flame Theme Header Styles
  // ==========================================
  .site-header {
    position: sticky;
    top: 0;
    z-index: 100;
    background: var(--bg-card, var(--color-raised-bg));
    border-bottom: 1px solid var(--color-divider);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);

    // Subtle top glow effect
    &::before {
      content: "";
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 1px;
      background: linear-gradient(
        90deg,
        transparent 0%,
        var(--flame, #f16436) 50%,
        transparent 100%
      );
      opacity: 0.5;
    }
  }

  .header-container {
    max-width: 1400px;
    margin: 0 auto;
    padding: 0 24px;
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 32px;
  }

  .header-logo {
    display: flex;
    align-items: center;
    color: var(--color-text-dark);
    transition:
      opacity 0.2s,
      transform 0.2s;
    flex-shrink: 0;

    svg,
    img {
      height: 28px;
      width: auto;
      max-width: 150px;
      object-fit: contain;
    }

    &:hover {
      opacity: 0.85;
      transform: scale(1.02);
    }
  }

  .header-nav {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    justify-content: center;
  }

  .nav-link {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 10px;
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--color-secondary);
    text-decoration: none;
    transition: all 0.2s ease;
    position: relative;
    background: transparent;
    border: none;
    cursor: pointer;

    svg {
      width: 18px;
      height: 18px;
      flex-shrink: 0;
    }

    &:hover {
      color: var(--color-text-dark);
      background: var(--accent-muted, rgba(241, 100, 54, 0.08));
    }

    &.active {
      color: var(--flame, #f16436);
      background: var(--accent-muted, rgba(241, 100, 54, 0.12));

      &::after {
        content: "";
        position: absolute;
        bottom: -1px;
        left: 50%;
        transform: translateX(-50%);
        width: 24px;
        height: 2px;
        background: var(--flame, #f16436);
        border-radius: 1px;
      }
    }
  }

  // More dropdown
  .nav-more {
    position: relative;

    .nav-more-trigger {
      svg {
        width: 14px;
        height: 14px;
        transition: transform 0.2s;
      }
    }

    &:hover {
      .nav-more-trigger svg {
        transform: rotate(180deg);
      }

      .nav-more-dropdown {
        opacity: 1;
        visibility: visible;
        transform: translateY(0);
      }
    }
  }

  .nav-more-dropdown {
    position: absolute;
    top: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%) translateY(-8px);
    min-width: 160px;
    background: var(--bg-card, var(--color-raised-bg));
    border: 1px solid var(--color-divider);
    border-radius: 12px;
    padding: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12);
    opacity: 0;
    visibility: hidden;
    transition: all 0.2s ease;
    z-index: 200;

    &::before {
      content: "";
      position: absolute;
      top: -8px;
      left: 0;
      right: 0;
      height: 8px;
    }
  }

  .dropdown-link {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--color-secondary);
    text-decoration: none;
    transition: all 0.15s ease;

    svg {
      width: 18px;
      height: 18px;
    }

    &:hover {
      color: var(--flame, #f16436);
      background: var(--accent-muted, rgba(241, 100, 54, 0.08));
    }
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: 10px;
    background: var(--bg-elevated, var(--color-button-bg));
    border: 1px solid var(--color-divider);
    color: var(--color-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;

    svg {
      width: 20px;
      height: 20px;
    }

    &:hover {
      color: var(--flame, #f16436);
      border-color: var(--flame, #f16436);
      background: var(--accent-muted, rgba(241, 100, 54, 0.08));
    }
  }

  .create-btn {
    background: var(--accent-muted, rgba(241, 100, 54, 0.1));
    border-color: transparent;
    color: var(--flame, #f16436);

    &:hover {
      background: var(--flame, #f16436);
      color: white;
    }
  }

  .notification-btn {
    position: relative;
  }

  .notification-dot {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 8px;
    height: 8px;
    background: var(--flame, #f16436);
    border-radius: 50%;
    border: 2px solid var(--bg-card, var(--color-raised-bg));
  }

  .user-menu {
    display: flex;
    align-items: center;
    cursor: pointer;
    background: transparent !important;

    // Use :deep() to pierce through scoped styles to child components
    :deep(.popup-container) {
      background: transparent !important;

      > button {
        all: unset !important;
        background: transparent !important;
        background-color: transparent !important;
        border: none !important;
        padding: 0 !important;
        margin: 0 !important;
        box-shadow: none !important;
        cursor: pointer !important;
        display: flex !important;
        align-items: center !important;
      }
    }

    :deep(button) {
      all: unset !important;
      background: transparent !important;
      background-color: transparent !important;
      border: none !important;
      padding: 0 !important;
      margin: 0 !important;
      box-shadow: none !important;
      cursor: pointer !important;
      display: flex !important;
      align-items: center !important;
    }

    :deep(.user-avatar),
    :deep(.avatar) {
      width: 36px !important;
      height: 36px !important;
      min-width: 36px !important;
      min-height: 36px !important;
      border-radius: 50% !important;
      border: 2px solid var(--color-divider) !important;
      transition: border-color 0.2s !important;
      background: transparent !important;
      background-color: transparent !important;
      box-shadow: none !important;

      &:hover {
        border-color: var(--flame, #f16436) !important;
      }
    }
  }

  .user-avatar,
  .user-menu .avatar {
    width: 36px !important;
    height: 36px !important;
    min-width: 36px !important;
    min-height: 36px !important;
    border-radius: 50% !important;
    border: 2px solid var(--color-divider) !important;
    transition: border-color 0.2s !important;
    background: transparent !important;
    background-color: transparent !important;
    box-shadow: none !important;

    &:hover {
      border-color: var(--flame, #f16436) !important;
    }
  }

  .login-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    background: var(--flame, #f16436);
    color: white;
    font-weight: 600;
    font-size: 0.9rem;
    border-radius: 10px;
    text-decoration: none;
    transition: all 0.2s ease;
    box-shadow: 0 2px 8px rgba(241, 100, 54, 0.3);

    svg {
      width: 18px;
      height: 18px;
    }

    &:hover {
      background: var(--flame-dark, #c94d28);
      transform: translateY(-1px);
      box-shadow: 0 4px 16px rgba(241, 100, 54, 0.4);
    }
  }

  // New Footer Styles
  .site-footer {
    margin-top: 80px;
    background: var(--bg-card, var(--color-raised-bg));
    border-top: 1px solid var(--color-divider);
  }

  .footer-container {
    max-width: 1400px;
    margin: 0 auto;
    padding: 60px 40px;
    display: grid;
    grid-template-columns: 1.5fr 2fr auto;
    gap: 60px;

    @media (max-width: 1024px) {
      grid-template-columns: 1fr;
      gap: 40px;
      padding: 40px 24px;
    }
  }

  .footer-brand {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .footer-logo {
    display: inline-block;
    color: var(--color-text-dark);
    transition: opacity 0.3s;

    &:hover {
      opacity: 0.8;
    }
  }

  .footer-tagline {
    font-size: 0.9rem;
    color: var(--color-secondary);
    line-height: 1.6;
    max-width: 280px;
    margin: 0;
  }

  .footer-qq-group {
    font-size: 0.85rem;
    color: var(--color-secondary);
    text-decoration: none;
    cursor: pointer;
    transition: color 0.2s ease;

    &:hover {
      color: var(--color-brand);
    }
  }

  .footer-social {
    display: flex;
    gap: 12px;
    margin-top: 8px;
  }

  .social-link {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    background: var(--bg-elevated, var(--color-button-bg));
    border-radius: 10px;
    color: var(--color-secondary);
    transition: all 0.3s;

    &:hover {
      background: var(--accent-muted, rgba(241, 100, 54, 0.12));
      color: var(--flame, #f16436);
      transform: translateY(-2px);
    }
  }

  .social-icon {
    width: 20px;
    height: 20px;
  }

  .footer-links {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 40px;

    @media (max-width: 640px) {
      grid-template-columns: repeat(2, 1fr);
      gap: 24px;
    }
  }

  .footer-column {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .footer-column-title {
    font-size: 0.8rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--color-text-dark);
    margin: 0 0 4px 0;
  }

  .footer-link {
    font-size: 0.9rem;
    color: var(--color-secondary);
    text-decoration: none;
    transition: color 0.2s;

    &:hover {
      color: var(--flame, #f16436);
    }
  }

  .footer-actions {
    display: flex;
    flex-direction: column;
    gap: 12px;

    @media (max-width: 1024px) {
      flex-direction: row;
    }
  }

  .theme-toggle,
  .settings-link {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    background: var(--bg-elevated, var(--color-button-bg));
    border: 1px solid var(--color-divider);
    border-radius: 12px;
    color: var(--color-text);
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.3s;
    text-decoration: none;

    svg {
      width: 18px;
      height: 18px;
    }

    &:hover {
      border-color: var(--flame, #f16436);
      background: var(--accent-muted, rgba(241, 100, 54, 0.08));
    }
  }

  .footer-bottom {
    border-top: 1px solid var(--color-divider);
    background: var(--bg-elevated, var(--color-bg));
  }

  .footer-bottom-content {
    max-width: 1400px;
    margin: 0 auto;
    padding: 24px 40px;
    text-align: center;

    @media (max-width: 640px) {
      padding: 20px 24px;
    }
  }

  .footer-disclaimer {
    font-size: 0.75rem;
    color: var(--color-secondary);
    line-height: 1.6;
    margin: 0 0 8px 0;
  }

  .footer-copyright {
    font-size: 0.75rem;
    color: var(--color-text-inactive);
    margin: 0;

    a {
      color: var(--color-text-inactive);
      text-decoration: none;
      transition: color 0.2s;

      &:hover {
        color: var(--color-secondary);
        text-decoration: underline;
      }
    }

    .police-beian {
      display: inline-flex;
      align-items: center;
      gap: 4px;

      img {
        width: 14px;
        height: 14px;
        vertical-align: middle;
      }
    }
  }
}

@media (min-width: 1024px) {
  .layout {
    main {
      .alpha-alert {
        margin: 1rem;

        .wrapper {
          padding: 1rem 2rem 1rem 1rem;
        }
      }
    }
  }
}

.email-nag {
  z-index: 6;
  position: relative;
  background-color: var(--color-raised-bg);
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  padding: 0.5rem 1rem;
}

.ban-nag {
  z-index: 6;
  position: relative;
  background-color: rgba(239, 68, 68, 0.15);
  border-bottom: 2px solid rgb(239, 68, 68);
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 0.5rem 1rem;
  color: var(--color-text);

  .ban-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: rgb(239, 68, 68);
    flex-shrink: 0;
  }

  span {
    font-size: 0.9rem;
  }

  .btn {
    background-color: rgb(239, 68, 68);
    color: white;
    padding: 0.25rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    font-weight: 500;
    text-decoration: none;
    transition: opacity 0.2s;

    &:hover {
      opacity: 0.9;
    }
  }
}

.site-banner--warning {
  // On some pages, there's gradient backgrounds that seep underneath
  // the banner, so we need to add a solid color underlay.
  background-color: black;
  border-bottom: 2px solid var(--color-red);
  display: grid;
  gap: 0.5rem;
  grid-template: "title actions" "description actions";
  padding-block: var(--gap-xl);
  padding-inline: max(calc((100% - 80rem) / 2 + var(--gap-md)), var(--gap-xl));
  z-index: 4;
  position: relative;

  &::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--color-red-bg);
    z-index: 5;
  }

  .site-banner__title {
    grid-area: title;
    display: flex;
    gap: 0.5rem;
    align-items: center;
    font-weight: bold;
    font-size: var(--font-size-md);
    color: var(--color-contrast);

    svg {
      color: var(--color-red);
      width: 1.5rem;
      height: 1.5rem;
      flex-shrink: 0;
    }
  }

  .site-banner__description {
    grid-area: description;
  }

  .site-banner__actions {
    grid-area: actions;
  }

  a {
    color: var(--color-red);
  }
}

@media (max-width: 1200px) {
  .app-btn {
    display: none;
  }
}

.mobile-navigation {
  display: none;

  .nav-menu {
    width: 100%;
    position: fixed;
    bottom: calc(var(--size-mobile-navbar-height) - var(--size-rounded-card));
    padding-bottom: var(--size-rounded-card);
    left: 0;
    background-color: var(--color-raised-bg);
    z-index: 11; // Upstream fix #4766: 20 = modals, 10 = svg icons
    transform: translateY(100%);
    transition: transform 0.4s cubic-bezier(0.54, 0.84, 0.42, 1);
    border-radius: var(--size-rounded-card) var(--size-rounded-card) 0 0;
    box-shadow: 0 0 20px 2px rgba(0, 0, 0, 0);

    .links,
    .account-container {
      display: grid;
      grid-template-columns: repeat(1, 1fr);
      grid-gap: 1rem;
      justify-content: center;
      padding: 1rem;

      .iconified-button {
        width: 100%;
        max-width: 500px;
        padding: 0.75rem;
        justify-content: center;
        font-weight: 600;
        font-size: 1rem;
        margin: 0 auto;
      }
    }

    .cascade-links {
      @media screen and (min-width: 354px) {
        grid-template-columns: repeat(2, 1fr);
      }

      @media screen and (min-width: 674px) {
        grid-template-columns: repeat(3, 1fr);
      }
    }

    &-browse {
      &.expanded {
        transform: translateY(0);
        box-shadow: 0 0 20px 2px rgba(0, 0, 0, 0.3);
      }
    }

    &-mobile {
      .account-container {
        padding-bottom: 0;

        .account-button {
          padding: var(--spacing-card-md);
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 0.5rem;

          .user-icon {
            width: 2.25rem;
            height: 2.25rem;
          }

          .account-text {
            flex-grow: 0;
          }
        }
      }

      &.expanded {
        transform: translateY(0);
        box-shadow: 0 0 20px 2px rgba(0, 0, 0, 0.3);
      }
    }
  }

  .mobile-navbar {
    display: flex;
    height: calc(var(--size-mobile-navbar-height) + env(safe-area-inset-bottom));
    border-radius: var(--size-rounded-card) var(--size-rounded-card) 0 0;
    padding-bottom: env(safe-area-inset-bottom);
    position: fixed;
    left: 0;
    bottom: 0;
    background-color: var(--color-raised-bg);
    box-shadow: 0 0 20px 2px rgba(0, 0, 0, 0.3);
    z-index: 11; // Upstream fix #4766: 20 = modals, 10 = svg icons
    width: 100%;
    align-items: center;
    justify-content: space-between;
    transition: border-radius 0.3s ease-out;
    border-top: 2px solid rgba(0, 0, 0, 0);
    box-sizing: border-box;

    &.expanded {
      box-shadow: none;
      border-radius: 0;
    }

    .tab {
      position: relative;
      background: none;
      display: flex;
      flex-basis: 0;
      justify-content: center;
      align-items: center;
      flex-direction: row;
      gap: 0.25rem;
      font-weight: bold;
      padding: 0;
      transition: color ease-in-out 0.15s;
      color: var(--color-text-inactive);
      text-align: center;

      &.browse {
        svg {
          transform: rotate(180deg);
          transition: transform ease-in-out 0.3s;

          &.closed {
            transform: rotate(0deg);
          }
        }
      }

      &.bubble {
        &::after {
          background-color: var(--color-brand);
          border-radius: var(--size-rounded-max);
          content: "";
          height: 0.5rem;
          position: absolute;
          left: 1.5rem;
          top: 0;
          width: 0.5rem;
        }
      }

      svg {
        height: 1.75rem;
        width: 1.75rem;

        &.smaller {
          width: 1.25rem;
          height: 1.25rem;
        }
      }

      .user-icon {
        width: 2rem;
        height: 2rem;
        transition: border ease-in-out 0.15s;
        border: 0 solid var(--color-brand);
        box-sizing: border-box;

        &.expanded {
          border: 2px solid var(--color-brand);
        }
      }

      &:hover,
      &:focus {
        color: var(--color-text);
      }

      &:first-child {
        margin-left: 2rem;
      }

      &:last-child {
        margin-right: 2rem;
      }

      &.router-link-exact-active:not(&.no-active) {
        svg {
          color: var(--color-brand);
        }

        color: var(--color-brand);
      }
    }
  }
}

@media (any-hover: none) and (max-width: 640px) {
  .desktop-only {
    display: none;
  }
}

@media (any-hover: none) and (max-width: 640px) {
  .mobile-navigation {
    display: flex;
  }

  main {
    padding-top: 0.75rem;
  }
}

.notification-link {
  position: relative;
}

.notification-badge {
  position: absolute;
  top: 0;
  right: 0;
  width: 8px;
  height: 8px;
  background-color: red;
  border-radius: 50%;
}
</style>
<style src="vue-multiselect/dist/vue-multiselect.css"></style>
