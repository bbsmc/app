<template>
  <div class="universal-card">
    <h2 class="text-2xl">{{ formatMessage(commonSettingsMessages.sessions) }}</h2>
    <p class="preserve-lines">
      {{ formatMessage(messages.sessionsDescription) }}
    </p>
    <div v-for="session in sessions" :key="session.id" class="universal-card recessed session mt-4">
      <div>
        <div>
          <strong>
            {{ session.os ?? formatMessage(messages.unknownOsLabel) }} ⋅
            {{ session.platform ?? formatMessage(messages.unknownPlatformLabel) }} ⋅
            {{ session.ip }}
          </strong>
        </div>
        <div>
          <template v-if="session.city">{{ session.city }}, {{ session.country }} ⋅ </template>
          <span
            v-tooltip="
              formatMessage(commonMessages.dateAtTimeTooltip, {
                date: new Date(session.last_login),
                time: new Date(session.last_login),
              })
            "
          >
            {{
              formatMessage(messages.lastAccessedAgoLabel, {
                ago: formatRelativeTime(session.last_login),
              })
            }}
          </span>
          ⋅
          <span
            v-tooltip="
              formatMessage(commonMessages.dateAtTimeTooltip, {
                date: new Date(session.created),
                time: new Date(session.created),
              })
            "
          >
            {{
              formatMessage(messages.createdAgoLabel, {
                ago: formatRelativeTime(session.created),
              })
            }}
          </span>
        </div>
      </div>
      <div class="input-group">
        <i v-if="session.current">{{ formatMessage(messages.currentSessionLabel) }}</i>
        <button v-else class="iconified-button raised-button" @click="revokeSession(session.id)">
          <XIcon /> {{ formatMessage(messages.revokeSessionButton) }}
        </button>
      </div>
    </div>
  </div>
</template>
<script setup>
import { XIcon } from "@modrinth/assets";
import { commonSettingsMessages } from "~/utils/common-messages.ts";

definePageMeta({
  middleware: "auth",
});

const { formatMessage } = useVIntl();
const formatRelativeTime = useRelativeTime();

const messages = defineMessages({
  currentSessionLabel: {
    id: "settings.sessions.current-session",
    defaultMessage: "当前会话",
  },
  revokeSessionButton: {
    id: "settings.sessions.action.revoke-session",
    defaultMessage: "撤销会话",
  },
  createdAgoLabel: {
    id: "settings.sessions.created-ago",
    defaultMessage: "创建于 {ago}",
  },
  sessionsDescription: {
    id: "settings.sessions.description",
    defaultMessage:
      "以下是当前登录您 BBSMC 账号的所有设备，您可以逐一注销。\n\n如果发现不认识的设备，请立即撤销该会话并修改密码。",
  },
  lastAccessedAgoLabel: {
    id: "settings.sessions.last-accessed-ago",
    defaultMessage: "上次访问 {ago}",
  },
  unknownOsLabel: {
    id: "settings.sessions.unknown-os",
    defaultMessage: "未知系统",
  },
  unknownPlatformLabel: {
    id: "settings.sessions.unknown-platform",
    defaultMessage: "未知平台",
  },
});

useHead({
  title: () => `${formatMessage(commonSettingsMessages.sessions)} - BBSMC资源社区`,
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const data = useNuxtApp();
const { data: sessions, refresh } = await useAsyncData("session/list", () =>
  useBaseFetch("session/list"),
);

async function revokeSession(id) {
  startLoading();
  try {
    sessions.value = sessions.value.filter((x) => x.id !== id);
    await useBaseFetch(`session/${id}`, {
      method: "DELETE",
    });
    await refresh();
  } catch (err) {
    data.$notify({
      group: "main",
      title: formatMessage(commonMessages.errorNotificationTitle),
      text: err.data.description,
      type: "error",
    });
  }
  stopLoading();
}
</script>
<style lang="scss" scoped>
.session {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;

  @media screen and (min-width: 800px) {
    flex-direction: row;
    align-items: center;

    .input-group {
      margin-left: auto;
    }
  }
}
</style>
