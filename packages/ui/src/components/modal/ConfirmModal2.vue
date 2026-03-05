<template>
  <NewModal ref="modal" :noblur="false" :on-hide="onHide">
    <template #title>
      <slot name="title">
        <span class="font-extrabold text-contrast text-lg">{{ title }}</span>
      </slot>
    </template>
    <div>
      <div class="markdown-body" v-html="renderString(description)" />
      <div class="flex gap-2 mt-6">
        <ButtonStyled color="blue">
          <button :disabled="action_disabled" @click="proceed">
            <StarIcon />
            {{ proceedLabel }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button @click="reject" style="margin-left: auto">
            <XIcon />
            {{ cancelLabel }}
          </button>
        </ButtonStyled>
      </div>
    </div>
  </NewModal>
</template>

<script setup>
import { renderString } from '@modrinth/utils'
import { ref } from 'vue'
import { StarIcon, XIcon } from '@modrinth/assets'
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
    default: 'No title defined',
    required: true,
  },
  description: {
    type: String,
    default: 'No description defined',
    required: true,
  },
  proceedLabel: {
    type: String,
    default: 'Proceed',
  },
  cancelLabel: {
    type: String,
    default: '拒绝',
  },
  onHide: {
    type: Function,
    default() {
      return () => {}
    },
  },
})

const emit = defineEmits(['proceed', 'reject'])
const modal = ref(null)

const action_disabled = ref(props.hasToType)

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

function reject() {
  if (modal.value) {
    modal.value.hide()
  }
  emit('reject')
}

function show() {
  if (modal.value) {
    modal.value.show()
  }
}

defineExpose({ show })
</script>
