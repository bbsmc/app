<script setup>
import { RightArrowIcon, InfoIcon } from "@modrinth/assets";
import { ButtonStyled } from "@modrinth/ui";

const emit = defineEmits(["navigate"]);

function handleNavigate() {
  emit("navigate");
}
</script>

<template>
  <div
    class="brand-gradient-bg card-shadow relative overflow-hidden rounded-2xl border-[1px] border-solid border-brand bg-bg p-4"
  >
    <InfoIcon
      class="absolute -right-12 -top-12 size-48 text-brand-highlight opacity-25 pointer-events-none"
      fill="none"
      stroke="var(--color-brand)"
      stroke-width="4"
    />
    <div class="relative z-10 flex items-center justify-between gap-4">
      <div class="flex flex-col gap-2">
        <span class="text-lg font-extrabold leading-tight text-contrast">
          需要<span class="text-brand">联机服务器</span>吗？
        </span>
        <span class="text-sm font-medium">
          稳定流畅、一键部署、仅需10分钟迅速联机本整合包
        </span>
        <span class="text-sm font-medium text-brand">
          最低仅需 58元/每月 即可畅快联机
        </span>
      </div>

      <!-- 查看详情按钮 -->
      <div class="flex items-center shrink-0">
        <ButtonStyled
          color="brand"
          class="cursor-pointer transition-transform hover:scale-105"
          @click="handleNavigate"
        >
          查看详情 <RightArrowIcon />
        </ButtonStyled>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.brand-gradient-bg {
  background: linear-gradient(135deg, var(--color-bg) 0%, rgba(var(--color-brand-rgb), 0.1) 100%);
}

.card-shadow {
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
}
</style>
