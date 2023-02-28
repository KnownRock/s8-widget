<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/tauri';

  async function getS8Value(){
    const result = await invoke('get_s8_value');
    return result as number;

    // return 1000

  }

  
  let s8Value = 0

  let green = 255;
  let red = 0;
  let style = `color: rgb(${red}, ${green}, 0);`;

  $: s8Value , (()=>{
    green = 255 - (s8Value - 1000) * 0.255;
    red = (s8Value - 1000) * 0.255;

    style = `color: rgb(${red}, ${green}, 0);`;
  })()
  

  onMount(() => {
    getS8Value().then((value) => {
      s8Value = value;
    });

    const interval = setInterval(() => {
      getS8Value().then((value) => {
        s8Value = value;
      });
    }, 10000);

    return () => {
      clearInterval(interval);
    };
  });
</script>

<div class="border">
  <div data-tauri-drag-region class="container" >
    <div  data-tauri-drag-region style="user-select: none;">
      <span data-tauri-drag-region  style={style}>{s8Value}</span>
    </div>
  </div>
</div>


<style>

</style>