<template>
  <div
    class="notification"
    :class="{
      'has-body': hasBody,
      compact: compact,
      read: notification.read,
    }"
  >
    <nuxt-link
      v-if="!type"
      :to="notification.link"
      class="notification__icon backed-svg"
      :class="{ raised: raised }"
    >
      <NotificationIcon />
    </nuxt-link>
    <nuxt-link
      v-else-if="type === 'profile_review_pending' || type === 'profile_review_result'"
      to="/settings/profile"
      class="notification__icon backed-svg"
      :class="{ raised: raised }"
    >
      <ModerationIcon class="moderation-color" />
    </nuxt-link>
    <div
      v-else-if="type === 'image_review_result'"
      class="notification__icon backed-svg"
      :class="{ raised: raised }"
    >
      <ModerationIcon class="moderation-color" />
    </div>
    <DoubleIcon v-else class="notification__icon">
      <template #primary>
        <nuxt-link v-if="project" :to="getProjectLink(project)" tabindex="-1">
          <Avatar size="xs" :src="project.icon_url" :raised="raised" no-shadow />
        </nuxt-link>
        <nuxt-link
          v-else-if="organization"
          :to="`/organization/${organization.slug}`"
          tabindex="-1"
        >
          <Avatar size="xs" :src="organization.icon_url" :raised="raised" no-shadow />
        </nuxt-link>
        <nuxt-link v-else-if="user" :to="getUserLink(user)" tabindex="-1">
          <Avatar size="xs" :src="user.avatar_url" :raised="raised" no-shadow />
        </nuxt-link>
        <Avatar v-else size="xs" :raised="raised" no-shadow />
      </template>
      <template #secondary>
        <ModerationIcon
          v-if="type === 'moderator_message' || type === 'status_change'"
          class="moderation-color"
        />
        <InvitationIcon v-else-if="type === 'team_invite' && project" class="creator-color" />
        <InvitationIcon
          v-else-if="type === 'organization_invite' && organization"
          class="creator-color"
        />
        <VersionIcon v-else-if="type === 'project_update' && project && version" />
        <StarIcon
          v-else-if="
            type === 'creator_application_message' ||
            type === 'creator_application_approved' ||
            type === 'creator_application_rejected'
          "
          class="creator-color"
        />
        <NotificationIcon v-else />
      </template>
    </DoubleIcon>
    <div class="notification__title">
      <template v-if="type === 'project_update' && project && version">
        您关注的项目
        <nuxt-link :to="getProjectLink(project)" class="title-link">{{ project.title }}</nuxt-link
        >, 已更新
      </template>
      <template v-else-if="type === 'team_invite' && project">
        <nuxt-link
          :to="`/user/${invitedBy.username}`"
          class="iconified-link title-link inline-flex"
        >
          <Avatar
            :src="invitedBy.avatar_url"
            circle
            size="xxs"
            no-shadow
            :raised="raised"
            class="inline-flex"
          />
          <span class="space">&nbsp;</span>
          <span>{{ invitedBy.username }}</span>
        </nuxt-link>
        <span>
          邀请您加入资源
          <nuxt-link :to="getProjectLink(project)" class="title-link">
            {{ project.title }} </nuxt-link
          >.
        </span>
      </template>
      <template v-else-if="type === 'organization_invite' && organization">
        <nuxt-link
          :to="`/user/${invitedBy.username}`"
          class="iconified-link title-link inline-flex"
        >
          <Avatar
            :src="invitedBy.avatar_url"
            circle
            size="xxs"
            no-shadow
            :raised="raised"
            class="inline-flex"
          />
          <span class="space">&nbsp;</span>
          <span>{{ invitedBy.username }}</span>
        </nuxt-link>
        <span>
          邀请您加入团队
          <nuxt-link :to="`/organization/${organization.slug}`" class="title-link">
            {{ organization.name }} </nuxt-link
          >.
        </span>
      </template>
      <template v-else-if="type === 'status_change' && project">
        <nuxt-link :to="getProjectLink(project)" class="title-link">
          {{ project.title }}
        </nuxt-link>
        <template v-if="tags.rejectedStatuses.includes(notification.body.new_status)">
          社区管理员已将
          <Badge :type="notification.body.new_status" />
        </template>
        <template v-else>
          从
          <Badge :type="notification.body.old_status" />
          更新到
          <Badge :type="notification.body.new_status" />
        </template>
      </template>

      <template v-else-if="type === 'wiki_cache'">
        <div v-if="notification.body.type_ === 'review'">
          您管理的资源
          <nuxt-link :to="getProjectLink(project) + '/wikis'" target="_blank" class="title-link"
            >{{ project.title }}
          </nuxt-link>
          有用户提交修改百科的请求，请前往审核
        </div>

        <div v-if="notification.body.type_ === 'reject'">
          您提交的百科修改
          <nuxt-link :to="getProjectLink(project) + '/wikis'" target="_blank" class="title-link"
            >{{ project.title }}
          </nuxt-link>
          被该资源管理员拒绝
          <br />
          <br />
          理由: {{ notification.body.msg }}
        </div>
        <div v-if="notification.body.type_ === 'accept'">
          您提交的百科修改
          <nuxt-link :to="getProjectLink(project) + '/wikis'" target="_blank" class="title-link"
            >{{ project.title }}
          </nuxt-link>
          已通过审核
          <br />
        </div>
        <div v-if="notification.body.type_ === 'time_out'">
          您提交的百科修改
          <nuxt-link :to="getProjectLink(project) + '/wikis'" target="_blank" class="title-link"
            >{{ project.title }}
          </nuxt-link>
          {{ notification.body.msg }}
          <br />
        </div>
      </template>

      <template v-else-if="type === 'forum'">
        <nuxt-link :to="`/user/${notification.body.sender}`" class="title-link">{{
          notification.body.sender
        }}</nuxt-link>
        在
        <nuxt-link
          v-if="notification.body.forum_type === 'project'"
          :to="`/project/${notification.body.project_id}/forum?id=${notification.body.number_of_posts}`"
          class="title-link"
        >
          {{ notification.body.forum_title }}
        </nuxt-link>
        <nuxt-link
          v-else
          :to="`/d/${notification.body.forum_id}?id=${notification.body.number_of_posts}`"
          class="title-link"
        >
          {{ notification.body.forum_title }}
        </nuxt-link>

        回复了您
        <br /><br />
      </template>

      <template v-else-if="type === 'moderator_message' && thread && project && !report">
        您的资源
        <nuxt-link :to="getProjectLink(project)" class="title-link">{{ project.title }}</nuxt-link
        >, 收到社区管理员的
        <template v-if="notification.grouped_notifs"> 消息 </template>
        <template v-else>消息</template>
      </template>

      <template v-else-if="type === 'moderator_message' && thread && report">
        社区管理员已回复您
        <template v-if="version">
          <nuxt-link :to="getVersionLink(project, version)" class="title-link">
            {{ version.name }}
          </nuxt-link>
          版本
        </template>
        <nuxt-link v-if="project" :to="getProjectLink(project)" class="title-link">
          {{ project.title }}
        </nuxt-link>
        <nuxt-link v-else-if="user" :to="getUserLink(user)" class="title-link">
          {{ user.username }} </nuxt-link
        >.
      </template>
      <template v-else-if="type === 'creator_application_message'">
        <nuxt-link to="/settings/creator" class="title-link">您的高级创作者申请</nuxt-link>
        有新回复
      </template>
      <template v-else-if="type === 'creator_application_approved'">
        🎉 恭喜！
        <nuxt-link to="/settings/creator" class="title-link">您的高级创作者申请</nuxt-link>
        已通过，您现在可以发布付费插件了！
      </template>
      <template v-else-if="type === 'creator_application_rejected'">
        <nuxt-link to="/settings/creator" class="title-link">您的高级创作者申请</nuxt-link>
        未通过<template v-if="notification.body.reason"
          >，原因：{{ notification.body.reason }}</template
        >
      </template>
      <template v-else-if="type === 'profile_review_pending'">
        <nuxt-link to="/settings/profile" class="title-link">
          {{ getReviewTypeName(notification.body.review_type) }}
        </nuxt-link>
        修改已提交审核
      </template>
      <template v-else-if="type === 'profile_review_result'">
        <nuxt-link to="/settings/profile" class="title-link">
          {{ getReviewTypeName(notification.body.review_type) }}
        </nuxt-link>
        修改审核结果<template v-if="notification.body.review_notes"
          >：{{ notification.body.review_notes }}</template
        >
      </template>
      <template v-else-if="type === 'image_review_result'">
        您上传的{{ getImageSourceTypeName(notification.body.source_type) }}因违规已被删除<template
          v-if="notification.body.review_notes"
          >，原因：{{ notification.body.review_notes }}</template
        >
      </template>
      <nuxt-link v-else :to="notification.link" class="title-link">
        <span v-html="renderString(notification.title)" />
      </nuxt-link>
      <!--      <span v-else class="known-errors">Error reading notification.</span>-->
    </div>

    <div v-if="hasBody" class="notification__body">
      <ThreadSummary
        v-if="type === 'moderator_message' && thread"
        :thread="thread"
        :link="threadLink"
        :raised="raised"
        :messages="getMessages()"
        class="thread-summary"
        :auth="auth"
      />
      <div v-else-if="type === 'project_update'" class="version-list">
        <div
          v-for="notif in (notification.grouped_notifs
            ? [notification, ...notification.grouped_notifs]
            : [notification]
          ).filter((x) => x.extra_data.version)"
          :key="notif.id"
          class="version-link"
        >
          <VersionIcon />
          <nuxt-link
            :to="getVersionLink(notif.extra_data.project, notif.extra_data.version)"
            class="text-link"
          >
            {{ notif.extra_data.version.name }}
          </nuxt-link>
          <span class="version-info">
            for
            <Categories
              :categories="notif.extra_data.version.loaders"
              :type="notif.extra_data.project.project_type"
              class="categories"
            />
            {{ $formatVersion(notif.extra_data.version.game_versions) }}
            <span v-tooltip="formatDateTime(notif.extra_data.version.date_published)" class="date">
              {{ fromNow(notif.extra_data.version.date_published) }}
            </span>
          </span>
        </div>
      </div>
      <template v-else>
        {{ notification.text }}
      </template>
    </div>

    <span class="notification__date">
      <span v-if="notification.read" class="read-badge inline-flex"> <ReadIcon /> 已读 </span>
      <span v-tooltip="formatDateTime(notification.created)" class="inline-flex">
        <CalendarIcon class="mr-1" /> 通知时间 {{ fromNow(notification.created) }}
      </span>
    </span>
    <div v-if="compact" class="notification__actions">
      <template v-if="type === 'team_invite' || type === 'organization_invite'">
        <button
          v-tooltip="`接受`"
          class="iconified-button square-button brand-button button-transparent"
          @click="
            () => {
              acceptTeamInvite(notification.body.team_id);
              read();
            }
          "
        >
          <CheckIcon />
        </button>
        <button
          v-tooltip="`拒绝`"
          class="iconified-button square-button danger-button button-transparent"
          @click="
            () => {
              removeSelfFromTeam(notification.body.team_id);
              read();
            }
          "
        >
          <CrossIcon />
        </button>
      </template>

      <button
        v-else-if="!notification.read"
        v-tooltip="`标记为已读`"
        class="iconified-button square-button button-transparent"
        @click="read()"
      >
        <CrossIcon />
      </button>
    </div>
    <div v-else class="notification__actions">
      <div v-if="type !== null" class="input-group">
        <template
          v-if="type === 'wiki_cache' && notification.body.type_ === 'review' && !notification.read"
        >
          <button
            class="iconified-button brand-button"
            @click="
              () => {
                router.push(`/project/${notification.body.project_id}/wikis`);
                read();
              }
            "
          >
            前往审核
          </button>
        </template>
        <template
          v-if="type === 'wiki_cache' && notification.body.type_ === 'reject' && !notification.read"
        >
          <button class="iconified-button brand-button" @click="again">重新编辑百科提交</button>
        </template>
        <template v-if="type === 'forum' && !notification.read">
          <button
            class="iconified-button brand-button"
            @click="
              () => {
                if (notification.body.forum_type === 'project') {
                  router.push(
                    `/project/${notification.body.project_id}/forum?id=${notification.body.number_of_posts}`,
                  );
                } else {
                  router.push(
                    `/d/${notification.body.forum_id}?id=${notification.body.number_of_posts}`,
                  );
                }
                read();
              }
            "
          >
            前往查看
          </button>
        </template>
        <template
          v-if="(type === 'team_invite' || type === 'organization_invite') && !notification.read"
        >
          <button
            class="iconified-button brand-button"
            @click="
              () => {
                acceptTeamInvite(notification.body.team_id);
                read();
              }
            "
          >
            <CheckIcon /> 接受
          </button>
          <button
            class="iconified-button danger-button"
            @click="
              () => {
                removeSelfFromTeam(notification.body.team_id);
                read();
              }
            "
          >
            <CrossIcon /> 拒绝
          </button>
        </template>

        <button
          v-else-if="!notification.read"
          class="iconified-button"
          :class="{ 'raised-button': raised }"
          @click="read()"
        >
          <div v-if="type === 'wiki_cache' && notification.body.type_ === 'reject'">
            <CheckIcon /> 放弃修改并已读
          </div>
          <div v-else><CheckIcon /> 标记为已读</div>
        </button>
        <CopyCode v-if="flags.developerMode" :text="notification.id" />
      </div>
      <div v-else class="input-group">
        <nuxt-link
          v-if="notification.link && notification.link !== '#'"
          class="iconified-button"
          :class="{ 'raised-button': raised }"
          :to="notification.link"
          target="_blank"
        >
          <ExternalIcon />
          打开链接
        </nuxt-link>

        <button
          v-for="(action, actionIndex) in notification.actions"
          :key="actionIndex"
          class="iconified-button"
          :class="{ 'raised-button': raised }"
          @click="performAction(notification, actionIndex)"
        >
          <CheckIcon v-if="action.title === '接受'" />
          <CrossIcon v-else-if="action.title === '拒绝'" />
          {{ action.title }}
        </button>
        <button
          v-if="notification.actions.length === 0 && !notification.read"
          class="iconified-button"
          :class="{ 'raised-button': raised }"
          @click="performAction(notification, null)"
        >
          <CheckIcon /> 标记为已读
        </button>
        <CopyCode v-if="flags.developerMode" :text="notification.id" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { renderString } from "@modrinth/utils";
import { formatDateTime } from "@modrinth/utils";
import InvitationIcon from "~/assets/images/utils/user-plus.svg?component";
import ModerationIcon from "~/assets/images/sidebar/admin.svg?component";
import NotificationIcon from "~/assets/images/sidebar/notifications.svg?component";
import StarIcon from "~/assets/images/utils/star.svg?component";
import ReadIcon from "~/assets/images/utils/check-circle.svg?component";
import CalendarIcon from "~/assets/images/utils/calendar.svg?component";
import VersionIcon from "~/assets/images/utils/version.svg?component";
import CheckIcon from "~/assets/images/utils/check.svg?component";
import CrossIcon from "~/assets/images/utils/x.svg?component";
import ExternalIcon from "~/assets/images/utils/external.svg?component";
import ThreadSummary from "~/components/ui/thread/ThreadSummary.vue";
import { getProjectLink, getVersionLink } from "~/helpers/projects.js";
import { getUserLink } from "~/helpers/users.js";
import { acceptTeamInvite, removeSelfFromTeam } from "~/helpers/teams.js";
import { markAsRead } from "~/helpers/notifications.js";
import DoubleIcon from "~/components/ui/DoubleIcon.vue";
import Avatar from "~/components/ui/Avatar.vue";
import Badge from "~/components/ui/Badge.vue";
import CopyCode from "~/components/ui/CopyCode.vue";
import Categories from "~/components/ui/search/Categories.vue";
const data = useNuxtApp();
const app = useNuxtApp();
const emit = defineEmits(["update:notifications"]);
const router = useNativeRouter();
const props = defineProps({
  notification: {
    type: Object,
    required: true,
  },
  notifications: {
    type: Array,
    required: true,
  },
  raised: {
    type: Boolean,
    default: false,
  },
  compact: {
    type: Boolean,
    default: false,
  },
  auth: {
    type: Object,
    required: true,
  },
});

const flags = useFeatureFlags();
const tags = useTags();

const type = computed(() =>
  !props.notification.body || props.notification.body.type === "legacy_markdown"
    ? null
    : props.notification.body.type,
);
const thread = computed(() => props.notification.extra_data.thread);
const report = computed(() => props.notification.extra_data.report);
const project = computed(() => props.notification.extra_data.project);
const version = computed(() => props.notification.extra_data.version);
const user = computed(() => props.notification.extra_data.user);
const organization = computed(() => props.notification.extra_data.organization);
const invitedBy = computed(() => props.notification.extra_data.invited_by);

const getReviewTypeName = (reviewType) => {
  const types = { avatar: "头像", username: "用户名", bio: "简介" };
  return types[reviewType] || "资料";
};

const getImageSourceTypeName = (sourceType) => {
  const types = { markdown: "Markdown图片", gallery: "项目渲染图" };
  return types[sourceType] || "图片";
};

const threadLink = computed(() => {
  if (report.value) {
    return `/dashboard/report/${report.value.id}`;
  } else if (project.value) {
    return `${getProjectLink(project.value)}/moderation#messages`;
  }
  return "#";
});

const hasBody = computed(() => !type.value || thread.value || type.value === "project_update");
const fetchNotifications = inject("fetchNotifications");

async function read() {
  try {
    const ids = [
      props.notification.id,
      ...(props.notification.grouped_notifs
        ? props.notification.grouped_notifs.map((notif) => notif.id)
        : []),
    ];
    const updateNotifs = await markAsRead(ids);
    const newNotifs = updateNotifs(props.notifications);
    emit("update:notifications", newNotifs);
    fetchNotifications();
  } catch (err) {
    app.$notify({
      group: "main",
      title: "将通知标记为已读时出错",
      text: err.data ? err.data.description : err,
      type: "error",
    });
  }
}

async function again() {
  try {
    await useBaseFetch(
      `project/${project.value.slug}/wiki_submit_again/${props.notification.body.wiki_cache_id}`,
      { apiVersion: 3, method: "POST" },
    );
    app.$notify({
      group: "main",
      title: "成功",
      text: "重新编辑百科",
      type: "success",
    });
    await read();
    router.push(`/project/${project.value.slug}/wikis`);
  } catch (err) {
    app.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
    if (err.data.description.includes("已无法再次")) {
      await read();
    }
  }
}

async function performAction(notification, actionIndex) {
  startLoading();
  try {
    await read();

    if (actionIndex !== null) {
      await useBaseFetch(`${notification.actions[actionIndex].action_route[1]}`, {
        method: notification.actions[actionIndex].action_route[0].toUpperCase(),
      });
    }
  } catch (err) {
    app.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
  stopLoading();
}

function getMessages() {
  const messages = [];
  if (props.notification.body.message_id) {
    messages.push(props.notification.body.message_id);
  }
  if (props.notification.grouped_notifs) {
    for (const notif of props.notification.grouped_notifs) {
      if (notif.body.message_id) {
        messages.push(notif.body.message_id);
      }
    }
  }
  return messages;
}
</script>

<style lang="scss" scoped>
.notification {
  display: grid;
  grid-template:
    "icon title"
    "actions actions"
    "date date";
  grid-template-columns: min-content 1fr;
  grid-template-rows: min-content min-content min-content;
  gap: var(--spacing-card-sm);

  &.compact {
    grid-template:
      "icon title actions"
      "date date date";
    grid-template-columns: min-content 1fr auto;
    grid-template-rows: auto min-content;
  }

  &.has-body {
    grid-template:
      "icon title"
      "body body"
      "actions actions"
      "date date";
    grid-template-columns: min-content 1fr;
    grid-template-rows: min-content auto auto min-content;

    &.compact {
      grid-template:
        "icon title actions"
        "body body body"
        "date date date";
      grid-template-columns: min-content 1fr auto;
      grid-template-rows: min-content auto min-content;
    }
  }

  .label__title,
  .label__description,
  h1,
  h2,
  h3,
  h4,
  :deep(p) {
    margin: 0 !important;
  }

  .notification__icon {
    grid-area: icon;
  }

  .notification__title {
    grid-area: title;
    color: var(--color-heading);
    margin-block: auto;
    display: inline-block;
    vertical-align: middle;
    line-height: 1.25rem;

    .iconified-link {
      display: inline;

      img {
        vertical-align: middle;
        position: relative;
        top: -2px;
      }
    }
  }

  .notification__body {
    grid-area: body;

    .version-list {
      margin: 0;
      padding: 0;
      list-style-type: none;
      display: flex;
      flex-direction: column;
      flex-wrap: wrap;
      gap: var(--spacing-card-sm);
      grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));

      .version-link {
        display: flex;
        flex-direction: row;
        gap: var(--spacing-card-xs);
        align-items: center;
        flex-wrap: wrap;

        .version-info {
          display: contents;

          :deep(span) {
            color: var(--color-text);
          }

          .date {
            color: var(--color-text-secondary);
            font-size: var(--font-size-sm);
          }
        }
      }
    }
  }

  .notification__date {
    grid-area: date;
    color: var(--color-text-secondary);

    svg {
      vertical-align: top;
    }

    .read-badge {
      font-weight: bold;
      color: var(--color-text);
      margin-right: var(--spacing-card-xs);
    }
  }

  .notification__actions {
    grid-area: actions;
    display: flex;
    flex-direction: row;
    gap: var(--spacing-card-sm);
  }

  .notification__actions .iconified-button.square-button svg {
    margin-right: 0;
  }

  .unknown-type {
    color: var(--color-red);
  }

  .title-link {
    &:not(:hover) {
      text-decoration: none;
    }

    font-weight: bold;
  }

  .moderation-color {
    color: var(--color-orange);
  }

  .creator-color {
    color: var(--color-blue);
  }
}
</style>
