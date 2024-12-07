<template>
  <section class="normal-page__content">
    <div
      v-if="wiki.body"
      class="markdown-body card"
      v-html="renderHighlightedString(wiki.body || '')"
    />
  </section>
</template>

<script setup>
import { renderHighlightedString } from "~/helpers/highlight.js";

const props = defineProps({
  project: {
    type: Object,
    default() {
      return {};
    },
  },
  wikis: {
    type: Object,
    default() {
      return {};
    },
  },
});
const title = `${props.project.title} - WIKI`;
const description = `浏览 ${props.project.title} 个图片的WIKI页面`;
let wiki = ref(null);
useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
});

console.log(props.wikis)
if (props.wikis.is_editor === true){
  props.wikis.cache.cache.forEach((wiki_) => {
    if (wiki_.featured){
      wiki = wiki_;
    }
  });
}else {
  props.wikis.wikis.forEach((wiki_) => {
    if (wiki_.featured){
      wiki = wiki_;
    }
  });

}


console.log('wikis', props.wikis);


</script>




<style scoped lang="scss">

</style>