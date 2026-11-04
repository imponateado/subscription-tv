import { Component, computed, inject, signal, viewChild } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { DatePipe } from '@angular/common';
import { YouTubePlayer } from '@angular/youtube-player';

interface Video {
  id: string;
  title: string;
  channel: string;
  published: string;
}

@Component({
  selector: 'app-root',
  imports: [YouTubePlayer, DatePipe],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App {
  private http = inject(HttpClient);
  private player = viewChild(YouTubePlayer);

  protected days = signal(7);
  protected oldestFirst = signal(false);
  protected loading = signal(false);
  protected failed = signal(false);
  private videos = signal<Video[]>([]); // backend devolve mais novos primeiro
  protected currentId = signal<string | null>(null);

  protected queue = computed(() => (this.oldestFirst() ? [...this.videos()].reverse() : this.videos()));
  private index = computed(() => this.queue().findIndex((v) => v.id === this.currentId()));
  protected current = computed(() => this.queue()[this.index()]);

  protected width = Math.min(960, window.innerWidth - 32);
  protected height = Math.round((this.width * 9) / 16);

  load() {
    const since = new Date(Date.now() - this.days() * 86_400_000).toISOString();
    this.loading.set(true);
    this.failed.set(false);
    this.http.get<Video[]>('/api/videos', { params: { since } }).subscribe({
      next: (v) => {
        this.videos.set(v);
        this.currentId.set(this.queue()[0]?.id ?? null);
        this.loading.set(false);
      },
      error: () => {
        this.failed.set(true);
        this.loading.set(false);
      },
    });
  }

  step(delta: number) {
    const v = this.queue()[this.index() + delta];
    if (v) this.currentId.set(v.id);
  }

  onState(e: YT.OnStateChangeEvent) {
    if (e.data === YT.PlayerState.ENDED) this.step(1);
    // trocar o videoId no <youtube-player> só deixa o vídeo "cued", então damos o play aqui
    if (e.data === YT.PlayerState.CUED) this.player()?.playVideo();
  }
}
