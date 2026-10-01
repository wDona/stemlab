"""uv run python test_engine.py"""
import engine

# voz: entre notas solapadas gana la más fuerte; las que no solapan se quedan
notes = [(0.0, 1.0, 60, 0.9), (0.2, 0.8, 72, 0.3), (1.0, 2.0, 62, 0.5)]
assert [n[2] for n in engine.monophonic(notes)] == [60, 62]

# 120 bpm: 0.5 s = una negra; 4 negras = un compás de 4/4 lleno
score = engine.notes_to_score([(i * 0.5, i * 0.5 + 0.5, 60 + i, 0.5) for i in range(4)], 120, engine.TRANSCRIBE["piano"], "t")
measures = score.parts[0].getElementsByClass("Measure")
assert len(measures) == 1, len(measures)
assert [n.pitch.midi for n in measures[0].notes] == [60, 61, 62, 63]
assert all(n.quarterLength == 1 for n in measures[0].notes)
print("ok")
