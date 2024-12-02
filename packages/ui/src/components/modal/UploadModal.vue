<template>
  <NewModal ref="modal" :noblur="noblur" :closeOnEsc="false" :closable="false" :on-hide="onHide">
    <template #title>
      <slot name="title">
        <span class="font-extrabold text-contrast text-lg">{{ title }}</span>
      </slot>
    </template>
    <div>
      <div class="markdown-body" v-html="renderString(description)" />
      <div style="margin-top: 15px" class="markdown-body" v-html="renderString(speed)" />
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
  title: {
    type: String,
    default: 'No title defined',
    required: true,
  },

  speed: {
    type: String,
    default: '--',
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
  noblur: {
    type: Boolean,
    default: false,
  },
  onHide: {
    type: Function,
    default() {
      return () => {
      }
    },
  },
})

const emit = defineEmits(['proceed'])
const modal = ref(null)

function proceed() {
  if (modal.value) {
    modal.value.hide()
  }
  emit('proceed')
}
function show() {
  if (modal.value) {
    modal.value.show()
  }
}


defineExpose({ show,proceed })
</script>
