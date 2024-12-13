import {Component, html, Model, Backend, timeout} from 'veda-client';
import Callback from './Callback.js';
import ClusterList from './ClusterList.js';
import ProcessList from './ProcessList.js';
import DocumentList from './DocumentList.js';
import DocumentProcessingPipelinesList from './DocumentProcessingPipelinesList.js';

export default class ProcessOverview extends Component(HTMLElement) {
  static tag = 'bpa-process-overview';

  activeTab = localStorage.getItem('ProcessOverview_activeTab') || 'processes';

  async added() {
    const params1 = new Model;
    params1['rdf:type'] = 'v-s:QueryParams';
    params1['v-s:storedQuery'] = 'v-bpa:OverallCounts';
    params1['v-s:resultFormat'] = 'cols';
    try {
      const {processes: [processesCount], documents: [documentsCount], processes_poorly_justified: [processesPoorlyJustifiedCount], processes_no_document: [processesNoDocumentCount]} = await Backend.stored_query(params1);
      this.processesCount = processesCount;
      this.documentsCount = documentsCount;
      this.processesPoorlyJustifiedCount = processesPoorlyJustifiedCount;
      this.processesNoDocumentCount = processesNoDocumentCount;
    } catch (e) {
      console.error('Error querying overall counts', e);
    }

    const params2 = new Model;
    params2['rdf:type'] = 'v-s:QueryParams';
    params2['v-s:storedQuery'] = 'v-bpa:CompletedAndRunningClusterizationAttempts';
    params2['v-s:resultFormat'] = 'cols';
    try {
      const {completed: [completed], running: [running]} = await Backend.stored_query(params2);
      this.completed = completed ? await new Model(completed).load() : null;
      this.running = running ? await new Model(running).load() : null;
      this.running?.on('modified', this.switchClusterList);
      Callback.set('ProcessOverview_setRunning', this.setRunning);
    } catch (e) {
      console.error('Error querying running clusterization attempts', e);
    }
  }

  setRunning = (running) => {
    this.running = running;
    this.running?.on('modified', this.switchClusterList);
  }

  switchClusterList = () => {
    if (this.running.hasValue('v-bpa:hasExecutionState', 'v-bpa:ExecutionCompleted')) {
      this.completed = this.running;
      this.running.off('modified', this.switchClusterList);
      this.update();
    }
  }

  removed() {
    this.running?.off('modified', this.switchClusterList);
    Callback.remove('ProcessOverview_setRunning');
  }

  toggleView(e) {
    this.activeTab = e.target.closest('button').id;
    localStorage.setItem('ProcessOverview_activeTab', this.activeTab);
    this.update();
  }

  render() {
    return html`
      <div class="mb-3 ms-3 d-flex justify-content-between align-items-center">
        <ul class="nav nav-underline">
          <li class="nav-item">
            <button id="documents" on:click="${(e) => this.toggleView(e)}" class="nav-link ${this.activeTab === 'documents' ? 'active disabled' : 'text-secondary-emphasis'}">
              <span about="v-bpa:ProcessDocuments" property="rdfs:label"></span>
              <!--span class="align-top badge bg-secondary">${this.documentsCount}</span-->
            </button>
          </li>
          <li class="nav-item">
            <button id="processes" on:click="${(e) => this.toggleView(e)}" class="nav-link ${this.activeTab === 'processes' ? 'active disabled' : 'text-secondary-emphasis'}">
              <span class="me-1" about="v-bpa:ShowProcesses" property="rdfs:label"></span>
              ${this.processesPoorlyJustifiedCount
                ? html`<span class="align-top badge bg-danger">${this.processesPoorlyJustifiedCount}</span>`
                : this.processesNoDocumentCount
                ? html`<span class="align-top badge bg-secondary">${this.processesNoDocumentCount}</span>`
                : ''
              }
            </button>
          </li>
          <li class="nav-item">
            <button id="clusters" on:click="${(e) => this.toggleView(e)}" class="nav-link ${this.activeTab === 'clusters' ? 'active disabled' : 'text-secondary-emphasis'}">
              <span class="me-1" about="v-bpa:ShowClusters" property="rdfs:label"></span>
              <span class="align-top badge bg-warning text-dark">${this.completed?.['v-bpa:foundClusters']?.length ?? 0}</span>
            </button>
          </li>
        </ul>
      </div>
      ${this.activeTab === 'clusters'
        ? html`
            <${ClusterList}
              ${this.completed ? html`about="${this.completed.id}"` : ''}
              ${this.running ? html`running="${this.running.id}"` : ''}
              callback="${Callback.getName(this.setRunning)}"
            >
            </${ClusterList}>
          `
        : this.activeTab === 'processes'
        ? html`<${ProcessList} poorly-justified="${this.processesPoorlyJustifiedCount}" no-document="${this.processesNoDocumentCount}"></${ProcessList}>`
        : html`
            <${DocumentProcessingPipelinesList}></${DocumentProcessingPipelinesList}>
            <${DocumentList}></${DocumentList}>
          `
      }
    `;
  }
}

customElements.define(ProcessOverview.tag, ProcessOverview);
