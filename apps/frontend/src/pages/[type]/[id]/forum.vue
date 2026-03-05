<template>
  <template v-if="project.forum">
    <ForumModal :discussion-id="project.forum" :is-project="true" />
  </template>

  <template v-else>
    <div class="normal-page__content markdown-body card">
      <h2>讨论</h2>
      <br />
      该资源作者没有开启讨论区
      <br />
      <div v-if="currentMember" class="mt-5 flex gap-2" style="justify-content: flex-end">
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
import { ButtonStyled } from "@modrinth/ui";
import { CheckIcon } from "@modrinth/assets";
import ForumModal from "~/components/ui/ForumModal.vue";
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

const title = `${props.project.title} 讨论区 - 我的世界资源交流 | BBSMC`;
const description = `参与 ${props.project.title} 的社区讨论，与其他玩家交流使用心得、报告问题和分享创意。在 BBSMC 加入活跃的 Minecraft 资源讨论社区。`;
useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
  ogImage: props.project.icon_url ?? "https://cdn.bbsmc.net/raw/placeholder.png",
});

async function openForumModal() {
  try {
    const res = await useBaseFetch(`project/${route.params.id}/forum`, {
      apiVersion: 3,
      method: "POST",
    });
    emit("update:project", { ...props.project, forum: res.id });
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
