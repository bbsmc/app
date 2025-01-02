<template>

  <template v-if="props.discussionId">
    <ForumModal :discussion-id="props.discussionId" />
  </template>

  <template v-else>
    <div class="normal-page__content markdown-body card">
      <h2>讨论</h2>
      <br />
      该资源作者没有开启讨论区
    </div>
  </template>
</template>


<script setup>
import ForumModal from "~/components/ui/ForumModal.vue";

const props = defineProps({
  project: {
    type: Object,
    default: () => ({}),
  },
  discussionId: {
    type: String,
    default: () => ("1yjdMOp"),
  },
});

</script>
<style scoped></style>
