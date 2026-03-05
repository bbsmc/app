<template>
  <NewModal ref="modal" :noblur="noblur" danger :on-hide="onHide">
    <template #title>
      <slot name="title">
        <span class="font-extrabold text-contrast text-lg">{{ title }}</span>
      </slot>
    </template>
    <div>
      <div class="markdown-body" v-html="renderString(description)" />
      <label v-if="hasToType" for="confirmation" class="confirmation-label">
        <span>
          <strong>请手动输入 </strong>
          <em class="confirmation-text">{{ confirmationText }} </em>
          <strong> 到下方输入框:</strong>
        </span>
      </label>
      <div class="confirmation-input">
        <input
          v-if="hasToType"
          id="confirmation"
          v-model="confirmation_typed"
          type="text"
          placeholder="在此处输入..."
          @input="type"
        />
      </div>
      <div class="flex gap-2 mt-6">
        <ButtonStyled color="red">
          <button :disabled="action_disabled" @click="proceed">
            <TrashIcon />
            {{ proceedLabel }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button @click="hide">
            <XIcon />
            取消
          </button>
        </ButtonStyled>
      </div>
    </div>
  </NewModal>
</template>

<script setup>
import { renderString } from '@modrinth/utils'
import { ref } from 'vue'
import { TrashIcon, XIcon } from '@modrinth/assets'
import NewModal from './NewModal.vue'
import ButtonStyled from '../base/ButtonStyled.vue'

const props = defineProps({
  confirmationText: {
    type: String,
    default: '',
  },
  hasToType: {
    type: Boolean,
    default: false,
  },
  title: {
    type: String,
    default: '未定义标题',
    required: true,
  },
  description: {
    type: String,
    default: '未定义描述',
    required: true,
  },
  proceedLabel: {
    type: String,
    default: '确认',
  },
  noblur: {
    type: Boolean,
    default: false,
  },
  onHide: {
    type: Function,
    default() {
      return () => {}
    },
  },
})

const emit = defineEmits(['proceed'])
const modal = ref(null)

const action_disabled = ref(props.hasToType)
const confirmation_typed = ref('')

function proceed() {
  if (modal.value) {
    modal.value.hide()
  }
  emit('proceed')
}

function hide() {
  if (modal.value) {
    modal.value.hide()
  }
}

function type() {
  if (props.hasToType) {
    action_disabled.value =
      confirmation_typed.value.toLowerCase() !== props.confirmationText.toLowerCase()
  }
}

function show() {
  if (modal.value) {
    modal.value.show()
  }
}

defineExpose({ show })
</script>
