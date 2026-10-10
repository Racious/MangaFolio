<script setup lang="ts">
import { useAppearanceStore } from "../stores/appearance";
import { ref } from "vue";
import { styles, validColor, type Appearance } from "../lib/appearance";
const appearance = useAppearanceStore();
const colorError = ref("");
function change<K extends keyof Appearance>(key: K, event: Event) {
  appearance.set(
    key,
    (event.target as HTMLSelectElement).value as Appearance[K],
  );
}
function setColor(key: keyof Appearance["colors"], event: Event) {
  const value = (event.target as HTMLInputElement).value;
  if (validColor(value)) { colorError.value = ""; appearance.set("colors", { ...appearance.settings.colors, [key]: value }); }
  else if (event.type === "change") { colorError.value = "請輸入六位色碼，例如 #346f65。"; (event.target as HTMLInputElement).value = appearance.settings.colors[key]; }
}
const colorFields = [{ key: "background", label: "背景" }, { key: "panel", label: "面板與選單" }, { key: "accent", label: "重點與按鈕" }] as const;
</script>
<template>
  <section class="appearance-settings" aria-label="外觀設定">
    <h2>外觀與閱讀</h2><p class="intro">版型決定排列與導覽，配色可另外調整。變更即時預覽並自動保存。</p>
    <div class="settings-fields">
      <label
        >介面風格<select
          aria-label="介面風格"
          :value="appearance.settings.style"
          @change="change('style', $event)"
        >
          <option
            v-for="style in styles"
            :key="style.value"
            :value="style.value"
          >
            {{ style.label }}
          </option>
        </select></label
      >
      <p>
        {{
          styles.find((s) => s.value === appearance.settings.style)?.description
        }}
      </p>
      <label
        >明暗主題<select
          aria-label="明暗主題"
          :value="appearance.settings.theme"
          @change="change('theme', $event)"
        >
          <option value="system">跟隨系統</option>
          <option value="light">明亮</option>
          <option value="dark">深色</option>
        </select></label
      >
      <div class="palette-settings">
        <label>配色方案<select aria-label="配色方案" :value="appearance.settings.palette" @change="change('palette', $event)">
          <option value="recommended">版型推薦</option><option value="jade">青玉</option><option value="plum">暖暮</option><option value="paper">紙韻</option><option value="custom">自訂顏色</option>
        </select></label>
        <p>明暗主題影響預設配色；自訂顏色在明亮與深色模式均使用您選的顏色。</p>
        <div v-if="appearance.settings.palette === 'custom'" class="custom-colors">
          <label v-for="field in colorFields" :key="field.key">{{ field.label }}
            <span><input type="color" :aria-label="`${field.label}顏色`" :value="appearance.settings.colors[field.key]" @input="setColor(field.key, $event)" />
            <input type="text" :aria-label="`${field.label}色碼`" :value="appearance.settings.colors[field.key]" maxlength="7" pattern="#[0-9a-fA-F]{6}" @input="setColor(field.key, $event)" @change="setColor(field.key, $event)" /></span>
          </label>
          <p>文字顏色會依背景自動調整。色碼使用 #RRGGBB 格式。</p>
          <p v-if="colorError" role="alert">{{ colorError }}</p>
        </div>
      </div>
      <label
        >書庫檢視<select
          aria-label="書庫檢視"
          :value="appearance.settings.view"
          @change="change('view', $event)"
        >
          <option value="grid">封面網格</option>
          <option value="detail">詳細列表</option>
          <option value="compact">緊湊列表</option>
        </select></label
      >
      <label
        >顯示密度<select
          aria-label="顯示密度"
          :value="appearance.settings.density"
          @change="change('density', $event)"
        >
          <option value="comfortable">舒適</option>
          <option value="compact">緊湊</option>
        </select></label
      >
      <label
        >網格封面尺寸<select
          aria-label="網格封面尺寸"
          :value="appearance.settings.coverSize"
          @change="change('coverSize', $event)"
        >
          <option value="small">小</option>
          <option value="medium">中</option>
          <option value="large">大</option>
        </select></label
      >
      <label><span><input type="checkbox" :checked="appearance.settings.readerPinned" @change="appearance.set('readerPinned', ($event.target as HTMLInputElement).checked)" /> 固定顯示閱讀工具列</span></label>
      <p v-if="appearance.saveFailed" role="status">
        設定暫時無法保存，本次仍可使用；重新啟動可能恢復預設。
      </p>
    </div>
  </section>
</template>
<style scoped>
summary {
  cursor: pointer;
  font-weight: 600;
  padding: 12px 0;
}
.settings-fields {
  display: grid;
  gap: 12px;
  padding-bottom: 16px;
}
label {
  display: grid;
  gap: 6px;
  font-size: 14px;
}
select {
  width: 100%;
}
p {
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-dim);
}
h2 { font-size: 22px; margin-bottom: 10px; }
.intro { margin-bottom: 22px; }
.settings-fields { max-width: 560px; gap: 18px; }
.custom-colors { display: grid; gap: 14px; margin-top: 14px; }
.custom-colors span { display: flex; gap: 10px; align-items: center; }
input[type=color] { width: 48px; height: 40px; padding: 3px; cursor: pointer; }
input[type=text] { width: 140px; }
</style>
