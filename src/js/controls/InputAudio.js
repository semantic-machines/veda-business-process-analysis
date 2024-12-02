import {Component, html, decorator} from 'veda-client';

export default class InputAudio extends Component(HTMLElement) {
  static tag = 'bpa-input-audio';

  added() {
    this.for = this.getAttribute('for');
  }

  render () {
    return html`
      <div class="d-flex align-items-center">
        <button class="cancel-button btn btn-link text-danger p-0 d-none">
          <i class="bi bi-x-circle-fill fs-5"></i>
        </button>
        <canvas class="audio-visualization mx-3 d-none" width="24" height="20"></canvas>
        <span class="recording-timer me-3 d-none">0.0</span>
        <button class="approve-button btn btn-link p-0 d-none">
          <i class="bi bi-check-circle-fill text-success fs-5"></i>
        </button>
        <button class="btn btn-link mic-button p-0" title="[Ии] Записать аудио и распознать текст">
          <i class="bi bi-mic-fill text-dark fs-5"></i>
        </button>
      </div>
    `;
  }

  post() {
    const input = document.getElementById(this.for);
    const micButton = this.querySelector('.mic-button');
    const canvas = this.querySelector('.audio-visualization');
    const canvasCtx = canvas.getContext('2d');
    const cancelButton = this.querySelector('.cancel-button');
    const approveButton = this.querySelector('.approve-button');
    const recordingTimer = this.querySelector('.recording-timer');

    let mediaRecorder;
    let audioChunks = [];
    let audioContext;
    let analyser;
    let startTime;
    let timerInterval;

    // Запрос доступа к микрофону
    const requestMicrophoneAccess = async () => {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({audio: true});
        return stream;
      } catch (error) {
        console.error('Ошибка доступа к микрофону:', error);
        alert('Ошибка доступа к микрофону: ' + error.message);
        throw error;
      }
    }

    // Визуализация интенсивности звука
    const startIntensityVisualization = (stream) => {
      // Создание аудиоконтекста
      audioContext = new (window.AudioContext || window.webkitAudioContext)();
      const source = audioContext.createMediaStreamSource(stream);
      analyser = audioContext.createAnalyser();
      source.connect(analyser);
      analyser.fftSize = 256;
      const bufferLength = analyser.frequencyBinCount;
      const audioDataArray = new Uint8Array(bufferLength);
      canvas.style.display = 'block';

      const sampleRate = audioContext.sampleRate;
      const minFrequency = 1000; // Минимальная частота голоса в Гц
      const maxFrequency = 3000; // Максимальная частота голоса в Гц
      const minBin = Math.floor(minFrequency / (sampleRate / analyser.fftSize));
      const maxBin = Math.ceil(maxFrequency / (sampleRate / analyser.fftSize));
      const numBars = 5;

      // Вычисляем ширину столбиков и расстояния между ними
      const barWidth = canvas.width / (numBars + (numBars - 1) * 0.62);
      const barSpacing = barWidth * 0.62;
      const halfHeight = canvas.height / 2;

      const draw = () => {
        if (!analyser) return;

        analyser.getByteFrequencyData(audioDataArray);

        // Очистка канваса
        canvasCtx.clearRect(0, 0, canvas.width, canvas.height);

        // Цвет фона
        canvasCtx.fillStyle = 'transparent';
        canvasCtx.fillRect(0, 0, canvas.width, canvas.height);

        // Рисуем столбики
        for (let i = 0; i < numBars; i++) {
          const startBin = Math.floor(minBin + i * ((maxBin - minBin) / numBars));
          const endBin = Math.ceil(minBin + (i + 1) * ((maxBin - minBin) / numBars));
          let sum = 0;

          // Рассчитываем среднюю амплитуду для текущего столбика
          for (let j = startBin; j < endBin; j++) {
            sum += audioDataArray[j];
          }
          const average = sum / (endBin - startBin);
          const barHeight = (average / 256) * halfHeight;
          const x = i * (barWidth + barSpacing);

          canvasCtx.fillStyle = `rgba(51, 122, 183, ${2 * barHeight / halfHeight})`;
          // Рисуем столбик вверх от середины
          canvasCtx.fillRect(x, halfHeight - barHeight, barWidth, barHeight);
          // Рисуем столбик вниз от середины
          canvasCtx.fillRect(x, halfHeight, barWidth, barHeight);
        }

        requestAnimationFrame(draw);
      }

      draw();
    }

    // Остановка визуализации звука
    const stopIntensityVisualization = () => {
      if (audioContext) {
        audioContext.close();
        audioContext = null;
      }
      if (analyser) {
        analyser.disconnect();
        analyser = null;
      }
    }

    // Обработчик событий для обновления таймера записи
    const startRecordingTimer = () => {
      startTime = Date.now();
      timerInterval = setInterval(() => {
        const elapsedTime = (Date.now() - startTime) / 1000;
        recordingTimer.textContent = elapsedTime.toFixed(1);
      }, 100);
    }

    const stopRecordingTimer = () => {
      clearInterval(timerInterval);
      recordingTimer.textContent = '0.0';
    }

    // Функция окончания записи
    const stopRecording = () => {
      return new Promise((resolve) => {
        mediaRecorder.onstop = resolve;
        mediaRecorder.stop();
        mediaRecorder.stream.getTracks().forEach((track) => track.stop());
      });
    }

    micButton.onclick = decorator(
      async (e) => {
        e.preventDefault();
        e.stopPropagation();
        audioChunks = [];
        const stream = await requestMicrophoneAccess();
        mediaRecorder = new MediaRecorder(stream);

        startIntensityVisualization(stream);
        startRecordingTimer();

        mediaRecorder.ondataavailable = (event) => {
          audioChunks.push(event.data);
        };

        mediaRecorder.start();
      },
      () => {
        hide(micButton);
        show(approveButton, cancelButton, recordingTimer, canvas);
      },
      null,
      (error) => {
        show(micButton);
        hide(approveButton, cancelButton, recordingTimer, canvas);
        console.error('Ошибка записи аудио:', error);
        alert('Ошибка записи аудио: ' + error.message);
      },
    );

    approveButton.onclick = decorator(
      async (e) => {
        e.preventDefault();
        e.stopPropagation();
        if (mediaRecorder && mediaRecorder.state === 'recording') {
          await stopRecording(); // Дождаться окончания записи
        }

        stopIntensityVisualization();
        stopRecordingTimer();

        try {
          await recognizeAudioFile(new Blob(audioChunks, {type: 'audio/ogg'}), (textChunk) => {
            const trimmed = textChunk.trim();
            const currentValue = input.value;
            let value;
            if (!currentValue) {
              value = trimmed;
            } else if (currentValue.endsWith('\n')) {
              value = currentValue + trimmed;
            } else {
              value = currentValue + '\n' + trimmed;
            }
            input.value = value;
            input.dispatchEvent(new Event('input', { bubbles: true }));
            input.dispatchEvent(new Event('change', { bubbles: true }));
          });
        } catch (error) {
          console.error('Ошибка распознавания аудио:', error);
          alert('Ошибка распознавания аудио: ' + error.message);
        } finally {
          audioChunks = [];
        }
      },
      () => {
        hide(approveButton, cancelButton, recordingTimer, canvas);
        show(micButton);
        micButton.disabled = true;
        micButton.firstElementChild.classList.remove('bi-mic-fill');
        micButton.firstElementChild.classList.add('spinner-grow', 'spinner-grow-sm');
      },
      () => {
        micButton.firstElementChild.classList.remove('spinner-grow', 'spinner-grow-sm');
        micButton.firstElementChild.classList.add('bi-mic-fill');
        micButton.disabled = false;
      },
    );

    // Обработчик для кнопки отмены
    cancelButton.onclick = decorator(
      async (e) => {
        e.preventDefault();
        e.stopPropagation();
        if (mediaRecorder && mediaRecorder.state === 'recording') {
          await stopRecording(); // Дождаться окончания записи
        }

        stopIntensityVisualization();
        stopRecordingTimer();

        // Очистка массива аудио чанков
        audioChunks = [];
      },
      () => {
        hide(approveButton, cancelButton, recordingTimer, canvas);
        show(micButton);
      },
    );
  }
}

customElements.define(InputAudio.tag, InputAudio);

async function recognizeAudioFile (file, fn) {
  const formData = new FormData();
  formData.append('file', file);

  const response = await fetch('/recognize_audio', {
    method: 'POST',
    body: formData,
    credentials: 'include',
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || response.status);
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let resultText = '';

  while (true) {
    const {done, value} = await reader.read();
    if (done) break;
    const decoded = decoder.decode(value, {stream: true});
    fn(decoded);
    resultText += decoded;
  }

  return resultText;
}

function show (...els) {
  els.forEach(el => el.classList.remove('d-none'));
}

function hide (...els) {
  els.forEach(el => el.classList.add('d-none'));
}
