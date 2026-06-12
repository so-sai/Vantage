<script lang="ts">
  let { originalCode = "", modifiedCode = "" } = $props<{
    originalCode?: string;
    modifiedCode?: string;
  }>();

  // Simple line-based diff generator (LCS/Match lookahead)
  let diffLines = $derived.by(() => {
    if (!originalCode && !modifiedCode) return [];
    
    const orig = originalCode.split('\n');
    const mod = modifiedCode.split('\n');
    
    let result: { type: 'normal' | 'addition' | 'deletion'; num?: number; char: string; text: string }[] = [];
    
    if (originalCode && !modifiedCode) {
      return orig.map((line, idx) => ({ type: 'deletion', num: idx + 1, char: '-', text: line }));
    }
    if (!originalCode && modifiedCode) {
      return mod.map((line, idx) => ({ type: 'addition', num: idx + 1, char: '+', text: line }));
    }

    let i = 0;
    let j = 0;
    while (i < orig.length || j < mod.length) {
      if (i < orig.length && j < mod.length) {
        if (orig[i].trim() === mod[j].trim()) {
          result.push({ type: 'normal', num: j + 1, char: ' ', text: mod[j] });
          i++;
          j++;
        } else {
          // Lookahead logic to find alignment match (max 4 lines)
          let foundMatch = false;
          for (let lookAhead = 1; lookAhead <= 4; lookAhead++) {
            if (i + lookAhead < orig.length && orig[i + lookAhead].trim() === mod[j].trim()) {
              for (let k = 0; k < lookAhead; k++) {
                result.push({ type: 'deletion', num: i + k + 1, char: '-', text: orig[i + k] });
              }
              i += lookAhead;
              foundMatch = true;
              break;
            }
            if (j + lookAhead < mod.length && orig[i].trim() === mod[j + lookAhead].trim()) {
              for (let k = 0; k < lookAhead; k++) {
                result.push({ type: 'addition', num: j + k + 1, char: '+', text: mod[j + k] });
              }
              j += lookAhead;
              foundMatch = true;
              break;
            }
          }
          
          if (!foundMatch) {
            result.push({ type: 'deletion', num: i + 1, char: '-', text: orig[i] });
            result.push({ type: 'addition', num: j + 1, char: '+', text: mod[j] });
            i++;
            j++;
          }
        }
      } else if (i < orig.length) {
        result.push({ type: 'deletion', num: i + 1, char: '-', text: orig[i] });
        i++;
      } else if (j < mod.length) {
        result.push({ type: 'addition', num: j + 1, char: '+', text: mod[j] });
        j++;
      }
    }
    
    return result;
  });
</script>

<div class="diff-container">
  {#if diffLines.length === 0}
    <div class="empty-state">
      <svg fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
      </svg>
      <h3>Chưa có Hiện vật thay đổi</h3>
      <p>Nhập ý định sửa lỗi vào thanh Spotlight phía trên để bắt đầu phân tích và hiển thị so sánh code.</p>
    </div>
  {:else}
    {#each diffLines as line}
      <div class="diff-line {line.type}">
        <span class="diff-num">{line.num !== undefined ? line.num : ''}</span>
        <span class="diff-char {line.type === 'addition' ? 'add' : line.type === 'deletion' ? 'del' : ''}">{line.char}</span>
        <span class="diff-code">{line.text}</span>
      </div>
    {/each}
  {/if}
</div>
