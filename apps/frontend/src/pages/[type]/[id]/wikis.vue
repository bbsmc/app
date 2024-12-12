<template>
  <ConfirmModal2
    ref="modal_confirm_create"
    title="您正在申请 创建/修改 百科页面"
    description="请认真预览的内容:<br/><br/>1. 请确保您的内容符合社区规范<br/>2. 请确保您的内容不违反任何法律法规<br/>3. 请确保您的内容不侵犯他人的知识产权<br/>4. 请确保您的内容不包含任何敏感信息
    <br/><br/>
    5.提交后您将获得该资源5小时编辑权限,请在此期间内完成编辑<br/>6.提交后您的内容将会被审核,审核通过后将会公开展示在资源页面上<br/>7.审核不通过的内容将会被删除,请确保您的内容符合以上要求
    <br/>8.在获得5小时权限期间和提交审核期间,其他人不能重复申请编辑权限，<br/>若未在规定时间内完成编辑，将会被禁止申请编辑权限48小时<br/>若审核不通过将会被禁止申请编辑权限72小时
    <br/><br/>请认真阅读并确认您的内容符合以上要求后再提交"
    proceed-label="确认"
    @proceed="submitCreateWiki()"
  />
<!--  <span style="margin-left: 10px"></span>-->

  <section class="normal-page__content">
    <div
      v-if="wiki"
      class="markdown-body card"
      v-html="renderHighlightedString(wiki.body || '')"
    />
    <div v-else-if="props.wikis.wikis.length === 0" class="universal-card">
      <div v-if="project.wiki_open || currentMember">
        <div v-if="props.wikis.is_editor">
          <h2>百科未设置任何页面为主页</h2>
          <br/>
          <span class="label__description">
          请在左边的目录中创建一个页面，并且将其设置为主页后，该页面将显示为你设置的页面的内容
        </span>
        </div>
        <div v-else>
          <h2>资源没有任何百科页面</h2>
          <br/>
          <span v-if="currentMember" class="label__description">
          您有权限编辑该资源的百科，您可以在此处创建百科页面
          </span>
          <span v-else class="label__description">
          当前资源的作者开启公开编辑百科的功能，您可以在此处创建百科页面
          </span>


          <br/>
          <br/>
          <button-styled color="green">
            <button @click="$refs.modal_confirm_create.show()" >
              <PlusIcon aria-hidden="true" />
              创建百科
            </button>
          </button-styled>
        </div>

      </div>
      <div v-else>
        <h2>百科</h2>
        <span class="label__description">
          该资源没有开启公开编辑百科的功能，作者也没自己创建百科页面
        </span>
      </div>

    </div>
  </section>
</template>

<script setup>
import { renderHighlightedString } from '~/helpers/highlight.js'
import { ButtonStyled } from '@modrinth/ui'
import { PlusIcon } from '@modrinth/assets'
import ConfirmModal2 from '@modrinth/ui/src/components/modal/ConfirmModal2.vue'
// import auth from '~/middleware/auth.js'
const auth = await useAuth();

const props = defineProps({
  project: {
    type: Object,
    default() {
      return {}
    }
  },
  currentMember: {
    type: Object,
    default() {
      return null;
    },
  },
  wikis: {
    type: Object,
    default() {
      return {}
    }
  }
})
const title = `${props.project.title} - WIKI`
const description = `浏览 ${props.project.title} 个图片的WIKI页面`
let wiki = ref(null)
let modal_confirm_create = ref()
const router = useNativeRouter()
const route = useNativeRoute()

const data = useNuxtApp()

useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description
})

if (props.wikis.is_editor === true) {
  props.wikis.cache.cache.forEach((wiki_) => {
    if (wiki_.featured) {
      wiki = wiki_
    }
    if (wiki_.child && wiki_.child.length > 0) {
      wiki_.child.forEach((wiki__) => {
        if (wiki__.featured) {
          wiki = wiki__
        }
      })
    }
  })
} else {
  props.wikis.wikis.forEach((wiki_) => {
    if (wiki_.featured) {
      wiki = wiki_
    }
    if (wiki_.child && wiki_.child.length > 0) {
      wiki_.child.forEach((wiki__) => {
        if (wiki__.featured) {
          wiki = wiki__
        }
      })
    }
  })
}



async function submitCreateWiki() {
  if (auth.value.user) {
    try {
      await useBaseFetch(`project/${route.params.id}/wiki_edit_start`, { apiVersion: 3, method: 'POST' })
      data.$notify({
        group: 'main',
        title: '成功',
        text: '</br>您已成功申请创建百科页面,请在五小时内提交审核',
        type: 'success'
      })
      router.push(`/project/${route.params.id}/wikis`)
      modal_confirm_create.value.hide()

    } catch (err) {
      console.log(err)
      data.$notify({
        group: 'main',
        title: '发生错误',
        text: err.data.description,
        type: 'error'
      })
    }
  } else {
    // auth.login()
    data.$notify({
      group: 'main',
      title: '未登录',
      text: '</br>请先登录或创建账号',
      type: 'error'
    })
    router.push(`/auth/sign-in`)
  }
  // console.log('submitCreateWiki')
}



</script>


<style scoped lang="scss">

</style>