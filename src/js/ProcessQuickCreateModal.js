// Start of Selection
import {Component, html, Model, genUri, decorator, timeout} from 'veda-client';
import Textarea from './controls/Textarea.js';
import InputAudio from './controls/InputAudio.js';
import {Modal} from 'bootstrap';

export default class ProcessQuickCreateModal extends Component(HTMLElement) {
  static tag = 'bpa-process-quick-create-modal';

  constructor() {
    super();
    this.model = new Model;
    this.model['rdf:type'] = 'v-bpa:GenericProcessingRequest';
    this.model['v-bpa:prompt'] = 'v-bpa:CreateBusinessProcessPrompt';
  }

  create = async () => {
    try {
      this.showSpinner(true);
      await this.createProcess();
    } catch (error) {
      this.handleError(error);
    } finally {
      this.showSpinner(false);
    }
  }

  createProcess = async () => {
    this.model.isSync(false);
    await this.model.save();
    await this.waitForProcessResult();
  }

  waitForProcessResult = async () => {
    return Promise.race([
      this.handleProcessResult(),
      this.createTimeout()
    ]);
  }

  handleProcessResult = () => {
    return new Promise((resolve, reject) => {
      const handleReset = async () => {
        if (!this.model.hasValue('v-bpa:hasResult')) return;

        try {
          const result = await this.model['v-bpa:hasResult'][0].load();
          const newProcess = this.prepareNewProcess(result);
          this.manualCreate(newProcess);
          resolve();
        } catch (error) {
          reject(error);
        } finally {
          this.model.off('afterreset', handleReset);
        }
      }

      this.model.on('afterreset', handleReset);
    });
  }

  prepareNewProcess = (result) => {
    const json = JSON.parse(JSON.stringify(result));
    json['@'] = genUri();
    json['rdf:type'] = json['v-bpa:targetType'];
    delete json['v-bpa:targetType'];

    const newProcess = new Model(json);
    newProcess.isNew(true);
    newProcess.isSync(false);
    return newProcess;
  }

  createTimeout = () => {
    return timeout(30000).then(() => {
      throw new Error('Превышено время ожидания обработки запроса');
    });
  }

  handleError = (error) => {
    alert(`Ошибка создания свободного описания процесса: ${error.message}`);
    console.error('Ошибка создания свободного описания процесса', error);
  }

  manualCreate(newProcess) {
    if (newProcess instanceof Event) {
      newProcess = new Model;
      newProcess['rdf:type'] = 'v-bpa:BusinessProcess';
    }
    Modal.getInstance(this.firstElementChild)?.hide();
    location.hash = `#/ProcessEdit/${newProcess.id}`;
  }

  cancel() {
    Modal.getInstance(this.firstElementChild)?.hide();
  }

  showSpinner(show) {
    const createButton = this.querySelector('.create-button');
    const spinner = createButton.querySelector('.spinner-grow');
    createButton.disabled = show;
    spinner.classList.toggle('d-none', !show);
  }

  removed() {
    Modal.getInstance(this.firstElementChild)?.hide();
  }

  post() {
    this.firstElementChild.addEventListener('shown.bs.modal', () => {
      this.querySelector('textarea')?.focus();
    });
  }

  render() {
    return html`
      <div class="modal fade" id="process-quick-create-modal" data-bs-keyboard="true">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-body">
              <div class="d-flex justify-content-between">
                <h3 class="mb-1">
                  <i class="bi bi-gear me-2"></i>
                  <span about="v-bpa:ProcessQuickCreate" property="rdfs:label"></span>
                </h3>
                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
              </div>
              <p class="text-muted fw-bold" about="v-bpa:ProcessQuickCreate" property="rdfs:comment"></p>
              <div class="mb-3 position-relative">
                <textarea id="quick-create-raw-input" class="form-control" placeholder="Введите текст с клавиатуры или воспользуйтесь микрофоном"
                  is="${Textarea}" about="${this.model.id}" property="v-bpa:rawInput" rows="7">
                </textarea>
                <div class="position-absolute bottom-0" style="right:0.75rem;">
                  <${InputAudio} for="quick-create-raw-input"></${InputAudio}>
                </div>
              </div>
              <div class="d-flex justify-content-between">
                <div class="d-flex gap-2">
                  <button on:click="${(e) => this.create(e)}" class="btn btn-primary create-button">
                    <span class="spinner-grow spinner-grow-sm me-2 d-none" aria-hidden="true"></span>
                    <span about="v-bpa:Create" property="rdfs:label"></span>
                  </button>
                  <button on:click="${(e) => this.cancel(e)}" class="btn btn-light">
                    <span about="v-bpa:Cancel" property="rdfs:label"></span>
                  </button>
                </div>
                <button on:click="${(e) => this.manualCreate(e)}" class="btn btn-light">
                  <span about="v-bpa:ManualCreate" property="rdfs:label"></span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;
  }
}

customElements.define(ProcessQuickCreateModal.tag, ProcessQuickCreateModal);
