<script setup>
import { RightArrowIcon, InfoIcon } from "@modrinth/assets";
import { ButtonStyled } from "@modrinth/ui";
import { ref, computed, watch } from "vue";

const props = defineProps({
  translationVersion: {
    type: [Object, Array],
    default: null,
  },
});

const emit = defineEmits(["navigate"]);

// 处理多个汉化包的情况
const translations = computed(() => {
  if (!props.translationVersion) return [];
  return Array.isArray(props.translationVersion) 
    ? props.translationVersion 
    : [props.translationVersion];
});

const selectedTranslation = ref(null);

// 监听 translations 变化，自动选中第一个
watch(translations, (newTranslations) => {
  if (newTranslations.length > 0 && !selectedTranslation.value) {
    selectedTranslation.value = newTranslations[0];
  }
}, { immediate: true });

function handleNavigate() {
  if (selectedTranslation.value) {
    emit("navigate", selectedTranslation.value);
  }
}

function selectTranslation(translation) {
  selectedTranslation.value = translation;
}

function getLanguageDisplay(code) {
  const languages = {
    'zh_CN': '简体中文',
    'zh_TW': '繁體中文',
    'en_US': 'English',
  };
  return languages[code] || code;
}
</script>

<template>
  <div
    v-if="translations.length > 0 && selectedTranslation"
    class="brand-gradient-bg card-shadow relative overflow-hidden rounded-2xl border-[1px] border-solid border-brand bg-bg p-4"
  >
    <InfoIcon
      class="absolute -right-12 -top-12 size-48 text-brand-highlight opacity-25 pointer-events-none"
      fill="none"
      stroke="var(--color-brand)"
      stroke-width="4"
    />
    <div class="relative z-10 flex flex-col gap-3">
      <div class="flex flex-col gap-2">
        <span class="text-lg font-extrabold leading-tight text-contrast">
          需要<span class="text-brand">汉化包</span>吗？
        </span>
        <span class="text-sm font-medium">
          已为此版本找到 {{ translations.length }} 个汉化包
        </span>
      </div>
      
      <!-- 汉化包选择区域 -->
      <div class="flex gap-2">
        <div class="flex-1">
          <!-- 单个汉化包时直接显示 -->
          <div v-if="translations.length === 1" class="rounded-lg border border-button-bg bg-raised p-3">
            <div class="flex items-center justify-between">
              <div class="flex-1">
                <div class="font-bold text-contrast">{{ selectedTranslation.project.title }}</div>
                <div class="text-xs text-secondary">
                  {{ getLanguageDisplay(selectedTranslation.language_code) }}
                  <span v-if="selectedTranslation.description"> · {{ selectedTranslation.description }}</span>
                </div>
              </div>
            </div>
          </div>
          
          <!-- 多个汉化包时显示可滚动列表 -->
          <div v-else class="max-h-32 overflow-y-auto rounded-lg border border-button-bg bg-raised">
            <div
              v-for="(translation, index) in translations"
              :key="index"
              class="cursor-pointer border-b border-button-bg p-2 last:border-b-0 hover:bg-button-bg"
              :class="{ 'bg-button-bg': selectedTranslation === translation }"
              @click="selectTranslation(translation)"
            >
              <div class="flex items-center gap-2">
                <input
                  type="radio"
                  :checked="selectedTranslation === translation"
                  class="h-4 w-4"
                  @click.stop="selectTranslation(translation)"
                />
                <div class="flex-1">
                  <div class="font-bold text-contrast">{{ translation.project.title }}</div>
                  <div class="text-xs text-secondary">
                    {{ getLanguageDisplay(translation.language_code) }}
                    <span v-if="translation.description"> · {{ translation.description }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 查看按钮 -->
        <div class="flex items-center">
          <ButtonStyled 
            color="brand" 
            class="cursor-pointer transition-transform hover:scale-105"
            @click="handleNavigate"
          >
            查看汉化包 <RightArrowIcon />
          </ButtonStyled>
        </div>
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