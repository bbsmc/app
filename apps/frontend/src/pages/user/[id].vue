<template>
  <div v-if="user" class="experimental-styles-within">
    <ModalCreation ref="modal_creation" />
    <CollectionCreateModal ref="modal_collection_creation" />
    <BanManageModal ref="banManageModal" :user-id="user.id" @updated="refreshUserData" />

    <!-- 用户详情模态框（超级管理员/社区管理员可见） -->
    <NewModal v-if="auth.user && isStaff(auth.user)" ref="userDetailsModal" header="用户详情">
      <div class="flex flex-col gap-3">
        <!-- Email（仅超级管理员可见） -->
        <div v-if="isAdmin(auth.user)" class="flex flex-col gap-1">
          <span class="text-lg font-bold text-primary">{{
            formatMessage(messages.emailLabel)
          }}</span>
          <div class="flex items-center gap-2">
            <span>{{ user.email || "未设置" }}</span>
            <span
              v-if="user.email"
              v-tooltip="
                user.email_verified
                  ? formatMessage(messages.emailVerifiedTooltip)
                  : formatMessage(messages.emailNotVerifiedTooltip)
              "
              class="flex items-center"
            >
              <CheckIcon v-if="user.email_verified" class="h-4 w-4 text-brand" />
              <XIcon v-else class="h-4 w-4 text-red" />
            </span>
          </div>
        </div>

        <!-- Email 验证状态（仅非超级管理员的社区管理员可见） -->
        <div v-if="!isAdmin(auth.user)" class="flex flex-col gap-1">
          <span class="text-lg font-bold text-primary">{{
            formatMessage(messages.emailVerifiedLabel)
          }}</span>
          <span class="flex w-fit items-center gap-1">
            <CheckIcon v-if="user.email_verified" class="h-4 w-4 text-brand" />
            <XIcon v-else class="h-4 w-4 text-red" />
            {{ user.email_verified ? "是" : "否" }}
          </span>
        </div>

        <!-- 认证方式（仅超级管理员可见） -->
        <div v-if="isAdmin(auth.user) && user.auth_providers" class="flex flex-col gap-1">
          <span class="text-lg font-bold text-primary">{{
            formatMessage(messages.authProvidersLabel)
          }}</span>
          <span>{{ user.auth_providers.join(", ") || "无" }}</span>
        </div>

        <!-- 是否设置密码（仅超级管理员可见） -->
        <div v-if="isAdmin(auth.user)" class="flex flex-col gap-1">
          <span class="text-lg font-bold text-primary">{{
            formatMessage(messages.hasPasswordLabel)
          }}</span>
          <span class="flex w-fit items-center gap-1">
            <CheckIcon v-if="user.has_password" class="h-4 w-4 text-brand" />
            <XIcon v-else class="h-4 w-4 text-red" />
            {{ user.has_password ? "是" : "否" }}
          </span>
        </div>

        <!-- 用户角色 -->
        <div class="flex flex-col gap-1">
          <span class="text-lg font-bold text-primary">用户角色</span>
          <span>{{ getUserRoleName(user.role) }}</span>
        </div>

        <!-- 用户 ID -->
        <div class="flex flex-col gap-1">
          <span class="text-lg font-bold text-primary">用户 ID</span>
          <span class="font-mono text-sm">{{ user.id }}</span>
        </div>
      </div>
    </NewModal>

    <div class="new-page sidebar" :class="{ 'alt-layout': cosmetics.leftContentLayout }">
      <div class="normal-page__header py-4">
        <ContentPageHeader>
          <template #icon>
            <Avatar :src="user.avatar_url" :alt="user.username" size="96px" circle />
          </template>
          <template #title>
            <span class="user-title-wrapper">
              {{ user.username }}
              <span
                v-if="
                  user.active_bans &&
                  user.active_bans.length > 0 &&
                  auth.user &&
                  tags.staffRoles.includes(auth.user.role)
                "
                v-tooltip="getBanTooltip(user.active_bans)"
                class="ban-badge"
              >
                <UserXIcon class="ban-icon" />
                已封禁
              </span>
            </span>
          </template>
          <template #summary>
            {{ user.bio ? user.bio : projects.length === 0 ? "BBSMC 用户" : "BBSMC 创作者" }}
          </template>
          <template #stats>
            <div
              class="flex items-center gap-2 border-0 border-r border-solid border-button-bg pr-4 font-semibold"
            >
              <BoxIcon class="h-6 w-6 text-secondary" />
              {{ formatCompactNumber(projects?.length || 0) }}
              个资源
            </div>
            <div
              class="flex items-center gap-2 border-0 border-r border-solid border-button-bg pr-4 font-semibold"
            >
              <DownloadIcon class="h-6 w-6 text-secondary" />
              {{ formatCompactNumber(sumDownloads) }}
              下载量
            </div>
            <div
              v-tooltip="
                formatMessage(commonMessages.dateAtTimeTooltip, {
                  date: new Date(user.created),
                  time: new Date(user.created),
                })
              "
              class="flex items-center gap-2 font-semibold"
            >
              <CalendarIcon class="h-6 w-6 text-secondary" />
              注册日期
              {{ formatDate(user.created) }}
            </div>
          </template>
          <template #actions>
            <ButtonStyled size="large">
              <NuxtLink v-if="auth.user && auth.user.id === user.id" to="/settings/profile">
                <EditIcon aria-hidden="true" />
                {{ formatMessage(commonMessages.editButton) }}
              </NuxtLink>
            </ButtonStyled>
            <ButtonStyled size="large" circular type="transparent">
              <OverflowMenu
                :options="[
                  {
                    id: 'user-details',
                    action: () => openUserDetailsModal(),
                    hoverOnly: true,
                    shown: auth.user && isStaff(auth.user) && auth.user.id !== user.id,
                  },
                  {
                    id: 'manage-bans',
                    action: () => openBanModal(),
                    color: 'red',
                    hoverOnly: true,
                    shown:
                      auth.user &&
                      tags.staffRoles.includes(auth.user.role) &&
                      auth.user.id !== user.id,
                  },
                  {
                    id: 'manage-projects',
                    action: () => navigateTo('/dashboard/projects'),
                    hoverOnly: true,
                    shown: auth.user && auth.user.id === user.id,
                  },
                  { divider: true, shown: auth.user && auth.user.id === user.id },
                  {
                    id: 'report',
                    action: () => (auth.user ? reportUser(user.id) : navigateTo('/auth/sign-in')),
                    color: 'red',
                    hoverOnly: true,
                    shown: auth.user?.id !== user.id,
                  },
                  { id: 'copy-id', action: () => copyId() },
                ]"
                aria-label="More options"
              >
                <MoreVerticalIcon aria-hidden="true" />
                <template #user-details>
                  <InfoIcon aria-hidden="true" />
                  用户详情
                </template>
                <template #manage-bans>
                  <UserXIcon aria-hidden="true" />
                  管理封禁
                </template>
                <template #manage-projects>
                  <BoxIcon aria-hidden="true" />
                  {{ formatMessage(messages.profileManageProjectsButton) }}
                </template>
                <template #report>
                  <ReportIcon aria-hidden="true" />
                  {{ formatMessage(commonMessages.reportButton) }}
                </template>
                <template #copy-id>
                  <ClipboardCopyIcon aria-hidden="true" />
                  {{ formatMessage(commonMessages.copyIdButton) }}
                </template>
              </OverflowMenu>
            </ButtonStyled>
          </template>
        </ContentPageHeader>
      </div>
      <div class="normal-page__content">
        <!-- 封禁横幅 -->
        <div v-if="user.active_bans && user.active_bans.length > 0" class="ban-banner">
          <UserXIcon class="ban-icon" />
          <span>
            该用户已被封禁（{{ getMainBanInfo?.typeName
            }}<template v-if="getMainBanInfo?.totalCount > 1"
              >等 {{ getMainBanInfo.totalCount }} 项</template
            >）。 封禁时间：{{ formatBanDate(getMainBanInfo?.bannedAt) }}。
            <template v-if="getMainBanInfo?.expiresAt">
              到期时间：{{ formatBanDate(getMainBanInfo.expiresAt) }}。
            </template>
            <template v-else>永久封禁。</template>
          </span>
        </div>
        <div v-if="navLinks.length > 2" class="mb-4 max-w-full overflow-x-auto">
          <NavTabs :links="navLinks" />
        </div>
        <div v-if="projects.length > 0">
          <div
            v-if="route.params.projectType !== 'collections'"
            :class="'project-list display-mode--' + cosmetics.searchDisplayMode.user"
          >
            <ProjectCard
              v-for="project in (route.params.projectType !== undefined
                ? projects.filter(
                    (x) =>
                      x.project_type ===
                      route.params.projectType.substr(0, route.params.projectType.length - 1),
                  )
                : projects
              )
                .slice()
                .sort((a, b) => b.downloads - a.downloads)"
              :id="project.slug || project.id"
              :key="project.id"
              :name="project.title"
              :display="cosmetics.searchDisplayMode.user"
              :featured-image="project.gallery.find((element) => element.featured)?.url"
              :description="project.description"
              :created-at="project.published"
              :updated-at="project.updated"
              :downloads="project.downloads.toString()"
              :follows="project.followers.toString()"
              :icon-url="project.icon_url"
              :categories="project.categories"
              :client-side="project.client_side"
              :server-side="project.server_side"
              :status="
                auth.user && (auth.user.id === user.id || tags.staffRoles.includes(auth.user.role))
                  ? project.status
                  : null
              "
              :type="project.project_type"
              :color="project.color"
            />
          </div>
        </div>
        <div v-else-if="route.params.projectType !== 'collections'" class="error">
          <UpToDate class="icon" /><br />
          <span v-if="auth.user && auth.user.id === user.id" class="preserve-lines text">
            <IntlFormatted :message-id="messages.profileNoProjectsAuthLabel">
              <template #create-link="{ children }">
                <a class="link" @click.prevent="$refs.modal_creation.show()">
                  <component :is="() => children" />
                </a>
              </template>
            </IntlFormatted>
          </span>
          <span v-else class="text">{{ formatMessage(messages.profileNoProjectsLabel) }}</span>
        </div>
        <div v-if="['collections'].includes(route.params.projectType)" class="collections-grid">
          <nuxt-link
            v-for="collection in collections"
            :key="collection.id"
            :to="`/collection/${collection.id}`"
            class="card collection-item"
          >
            <div class="collection">
              <Avatar :src="collection.icon_url" class="icon" />
              <div class="details">
                <h2 class="title">{{ collection.name }}</h2>
                <div class="stats">
                  <LibraryIcon aria-hidden="true" />
                  Collection
                </div>
              </div>
            </div>
            <div class="description">
              {{ collection.description }}
            </div>
            <div class="stat-bar">
              <div class="stats"><BoxIcon /> {{ collection.projects?.length || 0 }} projects</div>
              <div class="stats">
                <template v-if="collection.status === 'listed'">
                  <WorldIcon />
                  <span> Public </span>
                </template>
                <template v-else-if="collection.status === 'unlisted'">
                  <LinkIcon />
                  <span> Unlisted </span>
                </template>
                <template v-else-if="collection.status === 'private'">
                  <LockIcon />
                  <span> Private </span>
                </template>
                <template v-else-if="collection.status === 'rejected'">
                  <XIcon />
                  <span> Rejected </span>
                </template>
              </div>
            </div>
          </nuxt-link>
        </div>
        <div
          v-if="route.params.projectType === 'collections' && collections.length === 0"
          class="error"
        >
          <UpToDate class="icon" /><br />
          <span v-if="auth.user && auth.user.id === user.id" class="preserve-lines text">
            <IntlFormatted :message-id="messages.profileNoCollectionsAuthLabel">
              <template #create-link="{ children }">
                <a
                  class="link"
                  @click.prevent="(event) => $refs.modal_collection_creation.show(event)"
                >
                  <component :is="() => children" />
                </a>
              </template>
            </IntlFormatted>
          </span>
          <span v-else class="text">{{ formatMessage(messages.profileNoCollectionsLabel) }}</span>
        </div>

        <!-- 论坛内容区域 -->
        <div v-if="route.params.projectType === 'forum'" class="forum-content">
          <!-- 分类标签 -->
          <div class="mb-4">
            <NavTabs :links="forumTabLinks" query="tab" />
          </div>

          <!-- 加载状态 -->
          <div v-if="forumLoading" class="forum-loading">
            <span>加载中...</span>
          </div>

          <!-- 错误状态 -->
          <div v-else-if="forumError" class="forum-error">
            <span>{{ forumError }}</span>
          </div>

          <!-- 帖子列表 -->
          <div v-else-if="forumData && forumTab === 'discussions'" class="forum-list">
            <template v-if="forumData.discussions.length > 0">
              <nuxt-link
                v-for="discussion in forumData.discussions"
                :key="discussion.id"
                :to="
                  discussion.project_id
                    ? `/project/${discussion.project_id}/forum`
                    : `/d/${discussion.id}`
                "
                class="universal-card forum-card discussion-card"
              >
                <div class="card-type-badge discussion-badge">
                  <FileTextIcon class="badge-icon" />
                  {{ discussion.project_id ? "资源帖" : "帖子" }}
                </div>
                <div class="forum-header">
                  <Avatar
                    :src="user.avatar_url"
                    :alt="user.username"
                    size="40px"
                    class="user-avatar"
                  />
                  <div class="forum-info">
                    <h2 class="forum-title">{{ discussion.title }}</h2>
                    <div class="forum-meta">
                      <span class="meta-item">
                        <CalendarIcon class="meta-icon" />
                        {{ formatRelativeTime(discussion.created_at) }}
                      </span>
                      <span class="meta-item">
                        <MessageIcon class="meta-icon" />
                        {{ discussion.reply_count }} 回复
                      </span>
                      <span
                        v-if="discussion.category && !discussion.project_id"
                        class="meta-item category-tag"
                      >
                        {{ getCategoryName(discussion.category) }}
                      </span>
                    </div>
                  </div>
                </div>
                <div v-if="discussion.last_reply_content" class="forum-content-box">
                  <div class="content-label">
                    <MessageIcon class="label-icon" />
                    最新回复
                  </div>
                  <p class="content-text">{{ discussion.last_reply_content }}</p>
                </div>
                <div v-if="discussion.last_reply_time" class="forum-footer">
                  <div class="footer-right">
                    <span class="forum-last-post">
                      最后活动 {{ formatRelativeTime(discussion.last_reply_time) }}
                    </span>
                  </div>
                </div>
              </nuxt-link>
            </template>
            <div v-else class="forum-empty">
              <UpToDate class="icon" />
              <span class="text">暂无发表的帖子</span>
            </div>
          </div>

          <!-- 回复列表 -->
          <div v-else-if="forumData && forumTab === 'posts'" class="forum-list">
            <template v-if="forumData.posts.length > 0">
              <nuxt-link
                v-for="post in forumData.posts"
                :key="post.id"
                :to="
                  post.project_slug
                    ? `/project/${post.project_slug}/forum?id=${post.floor_number}`
                    : `/d/${post.discussion_id}?id=${post.floor_number}`
                "
                class="universal-card forum-card reply-card"
              >
                <div class="card-type-badge reply-badge">
                  <MessageIcon class="badge-icon" />
                  回复
                </div>
                <div class="forum-header">
                  <Avatar
                    :src="user.avatar_url"
                    :alt="user.username"
                    size="40px"
                    class="user-avatar"
                  />
                  <div class="forum-info">
                    <h2 class="forum-title">{{ post.discussion_title }}</h2>
                    <div class="forum-meta">
                      <span class="meta-item">
                        <CalendarIcon class="meta-icon" />
                        {{ formatRelativeTime(post.created_at) }}
                      </span>
                    </div>
                  </div>
                </div>
                <div class="forum-content-box my-reply-box">
                  <div class="content-label">
                    <EditIcon class="label-icon" />
                    我的回复
                  </div>
                  <p class="content-text">{{ post.content }}</p>
                </div>
                <div class="forum-footer">
                  <div class="footer-left">
                    <span class="view-thread-hint">
                      点击查看完整讨论
                      <ChevronRightIcon class="hint-icon" />
                    </span>
                  </div>
                </div>
              </nuxt-link>
            </template>
            <div v-else class="forum-empty">
              <UpToDate class="icon" />
              <span class="text">暂无发表的回复</span>
            </div>
          </div>

          <!-- 分页器 -->
          <div v-if="forumData && forumTotalPages > 1" class="forum-pagination">
            <button
              :disabled="forumPage <= 1"
              class="pagination-btn"
              @click="goToForumPage(forumPage - 1)"
            >
              <ChevronLeftIcon class="pagination-icon" />
              上一页
            </button>
            <span class="pagination-info"> 第 {{ forumPage }} / {{ forumTotalPages }} 页 </span>
            <button
              :disabled="forumPage >= forumTotalPages"
              class="pagination-btn"
              @click="goToForumPage(forumPage + 1)"
            >
              下一页
              <ChevronRightIcon class="pagination-icon" />
            </button>
          </div>
        </div>
      </div>
      <div class="normal-page__sidebar">
        <div v-if="organizations.length > 0" class="card flex-card">
          <h2 class="text-lg text-contrast">{{ formatMessage(messages.profileOrganizations) }}</h2>
          <div class="flex flex-wrap gap-2">
            <nuxt-link
              v-for="org in organizations"
              :key="org.id"
              v-tooltip="org.name"
              class="organization"
              :to="`/organization/${org.slug}`"
            >
              <Avatar :src="org.icon_url" :alt="'Icon for ' + org.name" size="3rem" />
            </nuxt-link>
          </div>
        </div>
        <div v-if="badges.length > 0" class="card flex-card">
          <h2 class="text-lg text-contrast">勋章</h2>
          <div class="flex flex-wrap gap-2">
            <div v-for="badge in badges" :key="badge">
              <StaffBadge v-if="badge === 'staff'" class="h-14 w-14" />
              <ModBadge v-else-if="badge === 'mod'" class="h-14 w-14" />
              <TenMClubBadge v-else-if="badge === '10m-club'" class="h-14 w-14" />
              <EarlyAdopterBadge v-else-if="badge === 'early-adopter'" class="h-14 w-14" />
              <AlphaTesterBadge v-else-if="badge === 'alpha-tester'" class="h-14 w-14" />
              <BetaTesterBadge v-else-if="badge === 'beta-tester'" class="h-14 w-14" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
<script setup>
import {
  LibraryIcon,
  BoxIcon,
  LinkIcon,
  LockIcon,
  XIcon,
  CalendarIcon,
  DownloadIcon,
  ClipboardCopyIcon,
  MoreVerticalIcon,
  MessageIcon,
  FileTextIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  CheckIcon,
  InfoIcon,
} from "@modrinth/assets";
import { OverflowMenu, ButtonStyled, ContentPageHeader, NewModal } from "@modrinth/ui";
import { isStaff, isAdmin } from "~/helpers/users.js";
import NavTabs from "~/components/ui/NavTabs.vue";
import ProjectCard from "~/components/ui/ProjectCard.vue";
import { reportUser } from "~/utils/report-helpers.ts";

import StaffBadge from "~/assets/images/badges/staff.svg?component";
import ModBadge from "~/assets/images/badges/mod.svg?component";
import TenMClubBadge from "~/assets/images/badges/10m-club.svg?component";
import EarlyAdopterBadge from "~/assets/images/badges/early-adopter.svg?component";
import AlphaTesterBadge from "~/assets/images/badges/alpha-tester.svg?component";
import BetaTesterBadge from "~/assets/images/badges/beta-tester.svg?component";

import ReportIcon from "~/assets/images/utils/report.svg?component";
import UpToDate from "~/assets/images/illustrations/up_to_date.svg?component";
import EditIcon from "~/assets/images/utils/edit.svg?component";
import WorldIcon from "~/assets/images/utils/world.svg?component";
import ModalCreation from "~/components/ui/ModalCreation.vue";
import Avatar from "~/components/ui/Avatar.vue";
import CollectionCreateModal from "~/components/ui/CollectionCreateModal.vue";
import BanManageModal from "~/components/ui/BanManageModal.vue";
import UserXIcon from "~/assets/images/utils/user-x.svg?component";

const data = useNuxtApp();
const route = useNativeRoute();
const auth = await useAuth();
const cosmetics = useCosmetics();
const tags = useTags();

const vintl = useVIntl();
const { formatMessage } = vintl;

const formatCompactNumber = useCompactNumber();

const formatRelativeTime = useRelativeTime();
const formatDate = (date) => data.$dayjs(date).format("YYYY-MM-DD");

const messages = defineMessages({
  profileProjectsStats: {
    id: "profile.stats.projects",
    defaultMessage: "<stat>{count}</stat> 个资源",
  },
  profileDownloadsStats: {
    id: "profile.stats.downloads",
    defaultMessage: "<stat>{count}</stat> 次下载",
  },
  profileProjectsFollowersStats: {
    id: "profile.stats.projects-followers",
    defaultMessage: "<stat>{count}</stat> 个关注者",
  },
  profileJoinedAt: {
    id: "profile.joined-at",
    defaultMessage: "注册于 <date>{ago}</date>",
  },
  profileUserId: {
    id: "profile.user-id",
    defaultMessage: "用户 ID: {id}",
  },
  profileDetails: {
    id: "profile.label.details",
    defaultMessage: "详情",
  },
  profileOrganizations: {
    id: "profile.label.organizations",
    defaultMessage: "组织",
  },
  profileBadges: {
    id: "profile.label.badges",
    defaultMessage: "勋章",
  },
  profileManageProjectsButton: {
    id: "profile.button.manage-projects",
    defaultMessage: "管理资源",
  },
  profileMetaDescription: {
    id: "profile.meta.description",
    defaultMessage:
      "访问 {username} 的 BBSMC 主页，浏览和下载 {username} 发布的 Minecraft 模组、整合包、光影和其他资源。加入 BBSMC 社区，发现更多创作者和资源。",
  },
  profileMetaDescriptionWithBio: {
    id: "profile.meta.description-with-bio",
    defaultMessage:
      "{bio} - 访问 {username} 的 BBSMC 主页，下载 {username} 发布的 Minecraft 模组、整合包和其他资源。",
  },
  profileNoProjectsLabel: {
    id: "profile.label.no-projects",
    defaultMessage: "该用户还没有资源！",
  },
  profileNoProjectsAuthLabel: {
    id: "profile.label.no-projects-auth",
    defaultMessage: "你还没有资源。\n想要 <create-link>创建一个</create-link> 吗？",
  },
  profileNoCollectionsLabel: {
    id: "profile.label.no-collections",
    defaultMessage: "该用户还没有收藏！",
  },
  profileNoCollectionsAuthLabel: {
    id: "profile.label.no-collections-auth",
    defaultMessage: "你还没有收藏。\n想要 <create-link>创建一个</create-link> 吗？",
  },
  userNotFoundError: {
    id: "profile.error.not-found",
    defaultMessage: "用户不存在",
  },
  // 用户详情模态框消息
  emailLabel: {
    id: "profile.details.label.email",
    defaultMessage: "邮箱",
  },
  emailVerifiedLabel: {
    id: "profile.details.label.email-verified",
    defaultMessage: "邮箱验证",
  },
  emailVerifiedTooltip: {
    id: "profile.details.tooltip.email-verified",
    defaultMessage: "邮箱已验证",
  },
  emailNotVerifiedTooltip: {
    id: "profile.details.tooltip.email-not-verified",
    defaultMessage: "邮箱未验证",
  },
  hasPasswordLabel: {
    id: "profile.details.label.has-password",
    defaultMessage: "是否设置密码",
  },
  authProvidersLabel: {
    id: "profile.details.label.auth-providers",
    defaultMessage: "认证方式",
  },
});

let user, projects, organizations, collections;
try {
  [{ data: user }, { data: projects }, { data: organizations }, { data: collections }] =
    await Promise.all([
      useAsyncData(`user/${route.params.id}`, () => useBaseFetch(`user/${route.params.id}`)),
      useAsyncData(
        `user/${route.params.id}/projects`,
        () => useBaseFetch(`user/${route.params.id}/projects`),
        {
          transform: (projects) => {
            if (!projects) return [];
            for (const project of projects) {
              project.categories = project.categories.concat(project.loaders);
              project.project_type = data.$getProjectTypeForUrl(
                project.project_type,
                project.categories,
                tags.value,
              );
            }

            return projects;
          },
        },
      ),
      useAsyncData(`user/${route.params.id}/organizations`, () =>
        useBaseFetch(`user/${route.params.id}/organizations`, {
          apiVersion: 3,
        }),
      ),
      useAsyncData(`user/${route.params.id}/collections`, () =>
        useBaseFetch(`user/${route.params.id}/collections`, { apiVersion: 3 }),
      ),
    ]);
} catch (err) {
  // 检查是否为 FetchError 并提取状态码
  const statusCode = err?.response?.status || err?.statusCode || err?.data?.statusCode || 404;
  const errorType = err?.data?.error;

  // 如果是限流错误，使用 429 状态码
  if (statusCode === 429 || errorType === "ratelimit_error") {
    throw createError({
      fatal: true,
      statusCode: 429,
      message: err?.data?.description || "请求过于频繁",
    });
  }

  // 其他错误使用原状态码或默认 404
  throw createError({
    fatal: true,
    statusCode,
    message: err?.data?.description || err?.message || formatMessage(messages.userNotFoundError),
  });
}

if (!user.value) {
  throw createError({
    fatal: true,
    statusCode: 404,
    message: formatMessage(messages.userNotFoundError),
  });
}

if (user.value.username !== route.params.id) {
  await navigateTo(`/user/${user.value.username}`, { redirectCode: 301 });
}

// 论坛内容状态
const forumData = ref(null);
const forumLoading = ref(false);
const forumError = ref(null);
const forumPage = ref(1);
const forumLimit = ref(20);

// 论坛标签 - 从路由 query 参数读取
const forumTab = computed(() => {
  const tab = route.query.tab;
  return tab === "posts" ? "posts" : "discussions";
});

// 论坛子标签链接
const forumTabLinks = computed(() => [
  {
    label: `发表的帖子${forumData.value ? ` (${forumData.value.total_discussions})` : ""}`,
    href: "",
    icon: FileTextIcon,
  },
  {
    label: `发表的回复${forumData.value ? ` (${forumData.value.total_posts})` : ""}`,
    href: "posts",
    icon: MessageIcon,
  },
]);

// 获取论坛内容
async function fetchForumContent(page = 1) {
  forumLoading.value = true;
  forumError.value = null;
  try {
    const data = await useBaseFetch(`user/${user.value.id}/forum`, {
      query: {
        page,
        limit: forumLimit.value,
      },
    });
    forumData.value = data;
    forumPage.value = page;
  } catch (err) {
    console.error("获取论坛内容失败:", err);
    forumError.value = err.message || "获取论坛内容失败";
  } finally {
    forumLoading.value = false;
  }
}

// 当路由为 forum 时自动加载论坛数据
watch(
  () => route.params.projectType,
  async (newType) => {
    if (newType === "forum" && !forumData.value) {
      await fetchForumContent();
    }
  },
  { immediate: true },
);

// 计算属性：总页数
const forumTotalPages = computed(() => {
  if (!forumData.value) return 1;
  const total =
    forumTab.value === "discussions"
      ? forumData.value.total_discussions
      : forumData.value.total_posts;
  return Math.ceil(total / forumLimit.value) || 1;
});

// 监听 tab 变化时只重置分页（数据已包含两种类型，无需重新请求）
watch(
  () => route.query.tab,
  () => {
    if (route.params.projectType === "forum") {
      forumPage.value = 1;
    }
  },
);

// 翻页
function goToForumPage(page) {
  if (page >= 1 && page <= forumTotalPages.value) {
    fetchForumContent(page);
  }
}

// 获取分类名称
function getCategoryName(category) {
  const categoryMap = {
    chat: "矿工茶馆",
    notice: "公告",
    project: "资源讨论",
    article: "专栏",
  };
  return categoryMap[category] || category;
}

const title = computed(
  () => `${user.value.username} 的主页 - BBSMC 我的世界资源社区 | 创作者资源下载`,
);
const description = computed(() =>
  user.value.bio
    ? formatMessage(messages.profileMetaDescriptionWithBio, {
        bio: user.value.bio,
        username: user.value.username,
      })
    : formatMessage(messages.profileMetaDescription, { username: user.value.username }),
);

useSeoMeta({
  title: () => title.value,
  description: () => description.value,
  ogTitle: () => title.value,
  ogDescription: () => description.value,
  ogImage: () => user.value.avatar_url ?? "https://cdn.bbsmc.net/raw/placeholder.png",
});

useHead({
  script: [
    {
      type: "application/ld+json",
      children: () =>
        JSON.stringify({
          "@context": "https://schema.org",
          "@type": "ProfilePage",
          mainEntity: {
            "@type": "Person",
            name: user.value.username,
            description: user.value.bio || undefined,
            image: user.value.avatar_url || undefined,
            url: `https://bbsmc.net/user/${user.value.username}`,
          },
        }),
    },
  ],
});

const projectTypes = computed(() => {
  const obj = {};

  if (collections.value?.length > 0) {
    obj.collection = true;
  }

  for (const project of projects.value ?? []) {
    obj[project.project_type] = true;
  }

  delete obj.project;

  return Object.keys(obj);
});
const sumDownloads = computed(() => {
  let sum = 0;

  for (const project of projects.value ?? []) {
    sum += project.downloads;
  }

  return sum;
});

const joinDate = computed(() => new Date(user.value.created));
const MODRINTH_BETA_END_DATE = new Date("2022-02-27T08:00:00.000Z");
const MODRINTH_ALPHA_END_DATE = new Date("2020-11-30T08:00:00.000Z");

const badges = computed(() => {
  const badges = [];

  if (user.value.role === "admin") {
    badges.push("staff");
  }

  if (user.value.role === "moderator") {
    badges.push("mod");
  }

  if (isPermission(user.value.badges, 1 << 0)) {
    badges.push("plus");
  }

  if (sumDownloads.value > 10000000) {
    badges.push("10m-club");
  }

  if (
    isPermission(user.value.badges, 1 << 1) ||
    isPermission(user.value.badges, 1 << 2) ||
    isPermission(user.value.badges, 1 << 3)
  ) {
    badges.push("early-adopter");
  }

  if (isPermission(user.value.badges, 1 << 4) || joinDate.value < MODRINTH_ALPHA_END_DATE) {
    badges.push("alpha-tester");
  } else if (isPermission(user.value.badges, 1 << 4) || joinDate.value < MODRINTH_BETA_END_DATE) {
    badges.push("beta-tester");
  }

  if (isPermission(user.value.badges, 1 << 5)) {
    badges.push("contributor");
  }

  if (isPermission(user.value.badges, 1 << 6)) {
    badges.push("translator");
  }

  return badges;
});

async function copyId() {
  await navigator.clipboard.writeText(user.value.id);
}

// 封禁管理模态框引用
const banManageModal = ref(null);

function openBanModal() {
  banManageModal.value?.show();
}

// 用户详情模态框引用
const userDetailsModal = ref(null);

function openUserDetailsModal() {
  userDetailsModal.value?.show();
}

// 获取用户角色名称
function getUserRoleName(role) {
  const roleNames = {
    admin: "超级管理员",
    moderator: "社区管理员",
    developer: "开发者",
    user: "用户",
  };
  return roleNames[role] || role;
}

async function refreshUserData() {
  // 重新获取用户数据以刷新封禁状态
  try {
    const updatedUser = await useBaseFetch(`user/${route.params.id}`);
    if (updatedUser) {
      user.value = updatedUser;
    }
  } catch (err) {
    console.error("刷新用户数据失败:", err);
  }
}

function getBanTooltip(bans) {
  if (!bans || bans.length === 0) return "";
  const banTypeNames = {
    global: "全局封禁",
    resource: "资源封禁",
    forum: "论坛封禁",
  };
  const banList = bans.map((ban) => {
    const typeName = banTypeNames[ban.ban_type] || ban.ban_type;
    return `${typeName}: ${ban.reason || "未提供原因"}`;
  });
  return banList.join("\n");
}

// 格式化封禁日期
function formatBanDate(dateStr) {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// 获取主要封禁信息用于横幅显示
const getMainBanInfo = computed(() => {
  if (!user.value?.active_bans || user.value.active_bans.length === 0) return null;

  const banTypeNames = {
    global: "全局封禁",
    resource: "资源封禁",
    forum: "论坛封禁",
  };

  // 优先显示全局封禁，其次资源封禁，最后论坛封禁
  const priorityOrder = ["global", "resource", "forum"];
  const sortedBans = [...user.value.active_bans].sort((a, b) => {
    return priorityOrder.indexOf(a.ban_type) - priorityOrder.indexOf(b.ban_type);
  });

  const mainBan = sortedBans[0];
  const typeName = banTypeNames[mainBan.ban_type] || mainBan.ban_type;

  return {
    typeName,
    bannedAt: mainBan.banned_at,
    expiresAt: mainBan.expires_at,
    totalCount: user.value.active_bans.length,
  };
});

const navLinks = computed(() => [
  {
    label: formatMessage(commonMessages.allProjectType),
    href: `/user/${user.value.username}`,
  },
  ...projectTypes.value
    .map((x) => {
      return {
        label: formatMessage(getProjectTypeMessage(x, true)),
        href: `/user/${user.value.username}/${x}s`,
      };
    })
    .slice()
    .sort((a, b) => a.label.localeCompare(b.label)),
  {
    label: "论坛动态",
    href: `/user/${user.value.username}/forum`,
  },
]);
</script>
<script>
export default defineNuxtComponent({
  methods: {},
});
</script>

<style lang="scss" scoped>
.collections-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);

  @media screen and (max-width: 800px) {
    grid-template-columns: repeat(1, 1fr);
  }

  gap: var(--gap-lg);

  .collection-item {
    display: flex;
    flex-direction: column;
    gap: var(--gap-md);
  }

  .description {
    // Grow to take up remaining space
    flex-grow: 1;

    color: var(--color-text);
    font-size: 16px;
  }

  .stat-bar {
    display: flex;
    align-items: center;
    gap: var(--gap-md);
    margin-top: auto;
  }

  .stats {
    display: flex;
    align-items: center;
    gap: var(--gap-xs);

    svg {
      color: var(--color-secondary);
    }
  }

  .collection {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--gap-md);

    .icon {
      width: 100% !important;
      height: 6rem !important;
      max-width: unset !important;
      max-height: unset !important;
      aspect-ratio: 1 / 1;
      object-fit: cover;
    }

    .details {
      display: flex;
      flex-direction: column;
      gap: var(--gap-sm);

      .title {
        color: var(--color-contrast);
        font-weight: 600;
        font-size: var(--font-size-lg);
        margin: 0;
      }
    }
  }
}

.user-title-wrapper {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.ban-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  background: var(--color-red-bg, rgba(239, 68, 68, 0.1));
  color: var(--color-red, #ef4444);
  border-radius: var(--radius-sm, 4px);
  font-size: 0.75rem;
  font-weight: 600;
  white-space: nowrap;
}

// 封禁横幅（与 default.vue 中的 ban-nag 风格一致）
.ban-banner {
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
  margin-bottom: 1rem;

  .ban-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: rgb(239, 68, 68);
    flex-shrink: 0;
  }

  span {
    font-size: 0.9rem;
  }
}

@media (max-width: 768px) {
  .ban-banner {
    flex-direction: column;
    text-align: center;
    gap: 0.5rem;
  }
}

// 论坛内容样式
.forum-content {
  width: 100%;
}

.forum-loading,
.forum-error {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--gap-xl);
  color: var(--color-secondary);
}

.forum-error {
  color: var(--color-red);
}

.forum-list {
  display: flex;
  flex-direction: column;
  gap: var(--gap-md);
}

// 论坛卡片样式
.forum-card {
  display: block;
  text-decoration: none;
  position: relative;
  padding: 1rem 1.25rem;
  border-radius: var(--radius-lg);
  transition: all 0.25s ease;
  background-color: var(--color-raised-bg);
  border: 1px solid transparent;

  &:hover {
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    transform: translateY(-2px);
    border-color: var(--color-brand-highlight);
  }
}

// 帖子卡片特定样式
.discussion-card {
  // 无左边框
}

// 回复卡片特定样式
.reply-card {
  // 无左边框
}

// 卡片类型标签
.card-type-badge {
  position: absolute;
  top: 0.75rem;
  right: 0.75rem;
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.7rem;
  font-weight: 600;

  .badge-icon {
    width: 0.875rem;
    height: 0.875rem;
  }
}

.discussion-badge {
  background: rgba(var(--color-brand-rgb), 0.15);
  color: var(--color-brand);
}

.reply-badge {
  background: rgba(var(--color-green-rgb), 0.15);
  color: var(--color-green);
}

.forum-header {
  display: flex;
  align-items: flex-start;
  gap: 0.875rem;
}

.user-avatar {
  flex-shrink: 0;
  border-radius: 50%;
}

.forum-info {
  flex-grow: 1;
  min-width: 0;
  padding-right: 3.5rem;
}

.reply-to-label {
  font-size: 0.7rem;
  color: var(--color-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 0.125rem;
}

.forum-title {
  font-size: 1rem;
  font-weight: 600;
  margin: 0;
  color: var(--color-contrast);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.4;
}

.forum-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.75rem;
  margin-top: 0.375rem;
  font-size: 0.8rem;
  color: var(--color-secondary);
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.meta-icon {
  width: 0.875rem;
  height: 0.875rem;
  opacity: 0.7;
}

.category-tag {
  padding: 0.125rem 0.5rem;
  background: var(--color-button-bg);
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
}

// 内容框样式
.forum-content-box {
  margin-top: 0.875rem;
  padding: 0.75rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-divider);
}

.my-reply-box {
  background: rgba(var(--color-green-rgb), 0.05);
  border-color: rgba(var(--color-green-rgb), 0.2);
}

.content-label {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-secondary);
  margin-bottom: 0.5rem;

  .label-icon {
    width: 0.875rem;
    height: 0.875rem;
    color: var(--color-brand);
  }
}

.my-reply-box .content-label .label-icon {
  color: var(--color-green);
}

.content-text {
  margin: 0;
  font-size: 0.875rem;
  line-height: 1.6;
  color: var(--color-text);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

// 底部栏
.forum-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 0.875rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-divider);
}

.footer-left,
.footer-right {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.state-tag {
  display: inline-flex;
  align-items: center;
  padding: 0.25rem 0.625rem;
  border-radius: var(--radius-max);
  font-size: 0.7rem;
  font-weight: 600;
}

.state-open {
  background: rgba(var(--color-green-rgb), 0.15);
  color: var(--color-green);
}

.state-closed {
  background: rgba(var(--color-secondary-rgb), 0.15);
  color: var(--color-secondary);
}

.forum-last-post {
  font-size: 0.75rem;
  color: var(--color-secondary);
}

.view-thread-hint {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.75rem;
  color: var(--color-secondary);
  transition: color 0.2s;

  .hint-icon {
    width: 0.875rem;
    height: 0.875rem;
    transition: transform 0.2s;
  }
}

.forum-card:hover .view-thread-hint {
  color: var(--color-brand);

  .hint-icon {
    transform: translateX(2px);
  }
}

.forum-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--gap-xl);
  text-align: center;

  .icon {
    width: 8rem;
    height: 8rem;
    margin-bottom: var(--gap-md);
    opacity: 0.5;
  }

  .text {
    color: var(--color-secondary);
    font-size: var(--font-size-md);
  }
}

.forum-pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--gap-md);
  margin-top: var(--gap-lg);
  padding-top: var(--gap-lg);
  border-top: 1px solid var(--color-button-bg);
}

.pagination-btn {
  display: flex;
  align-items: center;
  gap: var(--gap-xs);
  padding: var(--gap-sm) var(--gap-md);
  background: var(--color-button-bg);
  border: none;
  border-radius: var(--radius-md);
  color: var(--color-contrast);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: all 0.2s ease;

  &:hover:not(:disabled) {
    background: var(--color-brand);
    color: white;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .pagination-icon {
    width: 1rem;
    height: 1rem;
  }
}

.pagination-info {
  color: var(--color-secondary);
  font-size: var(--font-size-sm);
}

@media (max-width: 768px) {
  .forum-tabs {
    flex-direction: column;
  }

  .forum-tab {
    justify-content: center;
  }
}
</style>
