<template>
  <ReportView
    :auth="auth"
    :report-id="route.params.id"
    :breadcrumbs-stack="[{ href: '/moderation/reports', label: 'Reports' }]"
  />
</template>
<script setup>
import ReportView from "~/components/ui/report/ReportView.vue";

const auth = await useAuth();
const route = useNativeRoute();

useHead({
  title: `举报 ${route.params.id} - BBSMC`,
});
</script>
