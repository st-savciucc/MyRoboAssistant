export default async function encodeWav(webmBuf, sampleRate) {
  const ctx = new AudioContext();
  const audioBuf = await ctx.decodeAudioData(webmBuf);
  const chan = audioBuf.getChannelData(0); 
  const pcm16 = new Int16Array(chan.length);
  for (let i=0;i<chan.length;i++) pcm16[i] = Math.max(-1,Math.min(1,chan[i]))*32767;

  const wav = new Uint8Array(44 + pcm16.byteLength);
  const view = new DataView(wav.buffer);

  const writeStr = (o,s) => [...s].forEach((c,i)=>view.setUint8(o+i,c.charCodeAt(0)));
  writeStr(0,"RIFF"); view.setUint32(4, 36+pcm16.byteLength, true);
  writeStr(8,"WAVEfmt "); view.setUint32(16,16,true); view.setUint16(20,1,true);
  view.setUint16(22,1,true); view.setUint32(24,sampleRate,true);
  view.setUint32(28,sampleRate*2,true); view.setUint16(32,2,true); view.setUint16(34,16,true);
  writeStr(36,"data"); view.setUint32(40, pcm16.byteLength, true);
  new Uint8Array(wav.buffer,44).set(new Uint8Array(pcm16.buffer));
  return wav;
}
