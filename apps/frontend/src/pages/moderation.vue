<template>
  <div class="normal-page">
    <div class="normal-page__sidebar">
      <aside class="universal-card">
        <h1>版主</h1>
        <NavStack>
          <NavStackItem link="/moderation" label="信息统计">
            <ModrinthIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem link="/moderation/review" label="审核资源">
            <ModerationIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem link="/moderation/reports" label="举报">
            <ReportIcon aria-hidden="true" />
          </NavStackItem>
        </NavStack>
      </aside>
    </div>
    <div class="normal-page__content">
      <NuxtPage />
    </div>
  </div>
</template>
<script setup>
import NavStack from "~/components/ui/NavStack.vue";
import NavStackItem from "~/components/ui/NavStackItem.vue";

import ModrinthIcon from "~/assets/images/utils/users.svg?component";
import ModerationIcon from "~/assets/images/sidebar/admin.svg?component";
import ReportIcon from "~/assets/images/utils/report.svg?component";

definePageMeta({
  middleware: "auth",
});
</script>
