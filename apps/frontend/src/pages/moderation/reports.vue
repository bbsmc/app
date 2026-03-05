<template>
  <div>
    <section class="universal-card">
      <h2>举报管理</h2>
      <ReportsList :auth="auth" moderation />
    </section>
  </div>
</template>
<script setup>
import ReportsList from "~/components/ui/report/ReportsList.vue";

const auth = await useAuth();
useHead({
  title: "举报管理 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});
</script>
