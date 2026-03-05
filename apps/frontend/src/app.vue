<template>
  <NuxtLayout>
    <ModrinthLoadingIndicator />
    <Notifications />
    <NuxtPage />
  </NuxtLayout>
</template>
<script setup lang="ts">
import { provideNotificationManager } from "@modrinth/ui";
import ModrinthLoadingIndicator from "~/components/ui/modrinth-loading-indicator.ts";
import Notifications from "~/components/ui/Notifications.vue";
import { addNotification } from "~/composables/notifs.js";

// Provide notification manager for components that use injectNotificationManager
provideNotificationManager({
  addNotification,
});
</script>
