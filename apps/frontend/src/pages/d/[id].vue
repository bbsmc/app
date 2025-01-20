<template>
  <div class="game-header">
    <div class="hero-container">
      <img src="https://cdn.bbsmc.net/raw/top.jpeg" alt="header" />
      <div class="desktop-only"></div>
    </div>
  </div>

  <div class="normal-page">
    <div class="normal-page__sidebar">
      <div class="universal-card">
        <h2>论坛</h2>
        <NavStack>
          <NavStackItem link="/forums/notice" label="公告" />
          <NavStackItem link="/forums/chat" label="矿工茶馆" />
          <NavStackItem link="/forums/project" label="资源讨论" />
        </NavStack>
      </div>
    </div>
    <div class="normal-page__content">
      <div class="universal-card">
        <h2>{{ forum.title }}</h2>
        <!-- 分割线 -->
        <div class="divider"></div>
        <div class="user-info">
          <a :href="`/user/${forum.user_name}`" target="_blank">
            <img :src="forum.avatar" alt="user-avatar" class="user-avatar" />
          </a>

          <a :href="`/user/${forum.user_name}`" target="_blank">
            <span class="user-name">{{ forum.user_name }}</span>
          </a>

        </div>

        <div class="markdown-body" v-html="renderHighlightedString(forum.content)" />
      </div>
      <ForumModal :discussion-id="forumId" style="padding: 10px" />
    </div>
  </div>
</template>

<script setup>
import ForumModal from "~/components/ui/ForumModal.vue";
import NavStack from "~/components/ui/NavStack.vue";
import NavStackItem from "~/components/ui/NavStackItem.vue";
import { renderHighlightedString } from "~/helpers/highlight.js";

const route = useNativeRoute();

const forumId = route.params.id;
const forum = ref(null);

forum.value = await useBaseFetch(`forum/${forumId}`, {
  apiVersion: "3",
});

// console.log(forum.value);
</script>

<style scoped>
.hero-container {
  width: 100%;
  height: 144px;
  position: relative;
}

.game-header img {
  width: 100%;
  height: 144px;
}

.game-header .hero-container {
  height: 144px;
  z-index: 1;
}

.game-header .hero-container img {
  width: 100%;
  height: 144px;
  display: block;
}

body:has(.game-page) .game-header {
  margin-bottom: -110px;
  background-repeat: no-repeat;
}

body:has(.game-page) .game-header .hero-container:after {
  background: linear-gradient(hsla(0, 0%, 5%, 0.5), var(--color-background, #0d0d0d) 100%);
}

.game-header .hero-container:after {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: -1px;
  background: linear-gradient(0deg, #0d0d0d, transparent);
}

body:has(.normal-page) .game-header {
  margin-bottom: -110px;
  background-repeat: no-repeat;
}

.normal-page__content {
  padding: 20px;
  z-index: 2;
}

.normal-page__sidebar {
  padding: 20px;
  z-index: 2;
}

.user-info {
  display: flex;
  align-items: center;
  margin-bottom: 10px;
}

.user-avatar {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  margin-right: 10px;
}

.user-name {
  font-size: 1.2em;
  font-weight: bold;
}

/* 添加 universal-card 的悬停效果 */
.universal-card {
  transition:
    box-shadow 0.3s,
    transform 0.3s;
}

.universal-card:hover {
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
  transform: scale(1.02);
}

.divider {
  width: 100%;
  height: 1px;
  background-color: #e0e0e0;
  margin: 20px 0;
}
</style>
