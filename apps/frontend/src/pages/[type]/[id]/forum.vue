<template>

  <template v-if="project.forum">
    <ForumModal :discussion-id="project.forum" />
  </template>

  <template v-else>
    <div class="normal-page__content markdown-body card">
      <h2>讨论</h2>
      <br />
      该资源作者没有开启讨论区
      <br />
      <div class="mt-5 flex gap-2" style="justify-content: flex-end" v-if="currentMember">
        <ButtonStyled color="green">
          <button @click="openForumModal">
            <CheckIcon aria-hidden="true" />
            开启讨论区
          </button>
        </ButtonStyled>
      </div>
    </div>

  </template>
</template>


<script setup>
import ForumModal from "~/components/ui/ForumModal.vue";
import {
  ButtonStyled
} from "@modrinth/ui";
const data = useNuxtApp();
const router = useNativeRouter();
const route = useNativeRoute();

const props = defineProps({
  project: {
    type: Object,
    default: () => ({}),
  },
  currentMember: {
    type: Object,
    default() {
      return null;
    },
  },
});

async function openForumModal() {
  try {
    let res = await useBaseFetch(`project/${route.params.id}/forum`, {
      apiVersion: 3,
      method: "POST",
    });
    props.project.forum = res.id;
    data.$notify({
      group: "main",
      title: "成功",
      text: "开启讨论区",
      type: "success",
    });
    router.push(`/project/${route.params.id}/forum`);
  } catch (err) {
    console.log(err);
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
}
</script>
<style scoped></style>
