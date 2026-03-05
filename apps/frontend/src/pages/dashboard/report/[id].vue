<template>
  <ReportView
    :auth="auth"
    :report-id="route.params.id"
    :breadcrumbs-stack="[{ href: '/dashboard/reports', label: 'Active reports' }]"
  />
</template>
<script setup>
import ReportView from "~/components/ui/report/ReportView.vue";

const route = useNativeRoute();
const auth = await useAuth();

useHead({
  title: `举报详情 ${route.params.id} - BBSMC 我的世界资源社区`,
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});
</script>
