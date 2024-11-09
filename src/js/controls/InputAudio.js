import {Component, html, decorator} from 'veda-client';

export default class InputAudio extends Component(HTMLElement) {
  static tag = 'bpa-input-audio';

  property = this.dataset.property;

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
        <button class="btn btn-link mic-button p-0 text-dark" title="[Ии] Записать аудио и распознать текст">
          <i class="bi bi-mic-fill fs-5"></i>
        </button>
      </div>
    `;
  }

  post() {
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
    async function requestMicrophoneAccess () {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({audio: true});
        return stream;
      } catch (error) {
        alert('Доступ к микрофону запрещен: ' + error.message);
        throw error;
      }
    }

    // Визуализация интенсивности звука
    function startIntensityVisualization (stream) {
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

      async function draw () {
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
    function stopIntensityVisualization () {
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
    function startRecordingTimer () {
      startTime = Date.now();
      timerInterval = setInterval(() => {
        const elapsedTime = (Date.now() - startTime) / 1000;
        recordingTimer.textContent = elapsedTime.toFixed(1);
      }, 100);
    }

    function stopRecordingTimer () {
      clearInterval(timerInterval);
      recordingTimer.textContent = '0.0';
    }

    // Функция окончания записи
    function stopRecording () {
      return new Promise((resolve) => {
        mediaRecorder.onstop = resolve;
        mediaRecorder.stop();
        mediaRecorder.stream.getTracks().forEach((track) => track.stop());
      });
    }    

    micButton.onclick = async () => {
      if (micButton.firstElementChild.classList.contains('bi-mic-fill')) {
        try {
          micButton.disabled = true;
          audioChunks = [];
          const stream = await requestMicrophoneAccess();
          mediaRecorder = new MediaRecorder(stream);
  
          startIntensityVisualization(stream);
          startRecordingTimer();
  
          mediaRecorder.ondataavailable = (event) => {
            audioChunks.push(event.data);
          };
  
          mediaRecorder.start();
  
          micButton.classList.add('d-none');
          approveButton.classList.remove('d-none');
          cancelButton.classList.remove('d-none');
          recordingTimer.classList.remove('d-none');
          canvas.classList.remove('d-none');
  
          micButton.disabled = false;
        } catch (error) {
          // обратная смена свойств кнопки и отображение элементов управления
          micButton.disabled = false;
          console.error('Ошибка записи аудио:', error);
        }
      }
    }
  
    // Обработчик для кнопки "одобрения"
    approveButton.onclick = async () => {
      if (mediaRecorder && mediaRecorder.state === 'recording') {
        await stopRecording(); // Дождаться окончания записи
      }
  
      stopIntensityVisualization();
      stopRecordingTimer();
  
      // Очищаем элементы звуковой записи
      approveButton.classList.add('d-none');
      cancelButton.classList.add('d-none');
      recordingTimer.classList.add('d-none');
      canvas.classList.add('d-none');
  
      // Отображаем спиннер
      micButton.firstElementChild.classList.remove('bi-mic-fill');
      micButton.firstElementChild.classList.add('bi-arrow-clockwise', 'rotating');
      micButton.classList.remove('d-none');
      
      try {
        await decoratedRecognizeAudioFile.call(micButton, new Blob(audioChunks, {type: 'audio/ogg'}), (textChunk) => {
          const trimmed = textChunk.trim();
          const currentValue = getFilteredValue(this.model, this.property);
          let value;
          if (!currentValue) {
            value = trimmed;
          } else if (currentValue.endsWith('\n')) {
            value = currentValue + trimmed;
          } else {
            value = currentValue + '\n' + trimmed;
          }
          updateFilteredValue(this.model, this.property, value);
        });
      } catch (error) {
        console.error('Ошибка распознавания аудио:', error);
      } finally {
        audioChunks = [];
        micButton.firstElementChild.classList.remove('bi-arrow-clockwise', 'rotating');
        micButton.firstElementChild.classList.add('bi-mic-fill');
        micButton.disabled = false;
      }
    }
  
    // Обработчик для кнопки отмены
    cancelButton.onclick = async () => {
      if (mediaRecorder && mediaRecorder.state === 'recording') {
        await stopRecording(); // Дождаться окончания записи
      }
  
      stopIntensityVisualization();
      stopRecordingTimer();
  
      // Очистка массива аудио чанков
      audioChunks = [];
  
      // Сброс состояния кнопок
      micButton.firstElementChild.classList.remove('bi-stop');
      micButton.firstElementChild.classList.add('bi-mic-fill');
      micButton.disabled = false;
  
      micButton.classList.remove('d-none');
      approveButton.classList.add('d-none');
      cancelButton.classList.add('d-none');
      recordingTimer.classList.add('d-none');
      canvas.classList.add('d-none');
    }
  }
}

customElements.define(InputAudio.tag, InputAudio);

function getFilteredValue (model, property) {
  return model[property]
    ?.filter(str => !str.includes('^^') || str.toLowerCase().endsWith('^^' + document.documentElement.lang.toLowerCase()))
    .map(str => str.split('^^')[0])
    .join(' ') ?? '';
};

function updateFilteredValue (model, property, value) {
  const existingValues = model[property] || [];
  const currentLang = document.documentElement.lang.toUpperCase();
  const newValues = [...existingValues.filter(str => !str.endsWith(`^^${currentLang}`)), `${value}^^${currentLang}`];
  model[property] = newValues;
};

async function recognizeAudioFile (file, fn) {
  const formData = new FormData();
  formData.append('file', file);

  try {
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
  } catch (error) {
    console.error('Ошибка распознавания аудио:', error);
    throw error;
  }
}

// Показ спиннера на кнопке
function showSpinner (button) {
  const icon = button.firstElementChild;
  icon.classList.remove('bi-mic-fill', 'bi-stop');
  icon.classList.add('bi-arrow-clockwise', 'rotating');
  button.disabled = true;
}

// Скрытие спиннера на кнопке
function hideSpinner (button, iconClass) {
  const icon = button.firstElementChild;
  icon.classList.remove('bi-arrow-clockwise', 'rotating');
  icon.classList.add(iconClass);
  button.disabled = false;
}

// Обработка ошибок
async function handleError (error, button, errorIconClass) {
  hideSpinner(button, errorIconClass);
  alert('Произошла ошибка: ' + error);
}

// Декораторы для функций
const decoratedRecognizeAudioFile = decorator(
  recognizeAudioFile,
  function pre() { showSpinner(this); },
  function post() { hideSpinner(this, 'bi-mic-fill'); }, 
  function err(error) { handleError(error, this, 'bi-mic-fill'); }, 
);
