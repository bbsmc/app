<template>
  <div>
    <section class="universal-card">
      <h2 class="text-2xl">举报</h2>
      <ReportsList :auth="auth" />
    </section>
  </div>
</template>
<script setup>
import ReportsList from "~/components/ui/report/ReportsList.vue";

const auth = await useAuth();
useHead({
  title: "举报记录 - BBSMC 我的世界资源社区",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});
</script>
