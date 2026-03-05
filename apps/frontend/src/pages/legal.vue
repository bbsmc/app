<template>
  <div class="normal-page">
    <div class="normal-page__sidebar">
      <aside class="universal-card">
        <h1>法律条款</h1>
        <NavStack>
          <NavStackItem link="/legal/terms" label="用户服务协议">
            <HeartHandshakeIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem link="/legal/rules" label="内容规则">
            <ScaleIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem link="/legal/copyright" label="版权政策">
            <CopyrightIcon aria-hidden="true" />
          </NavStackItem>

          <h3>隐私</h3>
          <NavStackItem link="/legal/privacy" label="隐私保护声明">
            <LockIcon aria-hidden="true" />
          </NavStackItem>
        </NavStack>
      </aside>
    </div>
    <div class="normal-page__content">
      <NuxtPage class="universal-card" :route="route" />
    </div>
  </div>
</template>

<script setup>
import { HeartHandshakeIcon, LockIcon, ScaleIcon, CopyrightIcon } from "@modrinth/assets";
import NavStack from "~/components/ui/NavStack.vue";
import NavStackItem from "~/components/ui/NavStackItem.vue";

const route = useNativeRoute();
</script>

<style lang="scss" scoped>
.normal-page__content :deep(a) {
  color: var(--color-link);
  text-decoration: underline;

  &:focus-visible,
  &:hover {
    color: var(--color-link-hover);
  }

  &:active {
    color: var(--color-link-active);
  }
}
</style>
