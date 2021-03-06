package io.casperlabs.shared
import cats.effect.Sync
import com.github.ghik.silencer.silent
import io.casperlabs.metrics.Metrics

import scala.collection.mutable.{Map => MutableMap}

class MetricsTestImpl[F[_]: Sync] extends Metrics[F] {
  val counters: MutableMap[String, Long] = MutableMap.empty
  val samplers: MutableMap[String, Long] = MutableMap.empty
  val gauges: MutableMap[String, Long]   = MutableMap.empty
  @silent("The outer reference in this type test cannot be checked at run time.")
  private final case class Record(value: Long, count: Long)
  private val records: MutableMap[String, List[Record]] = MutableMap.empty

  private def incrementBy(name: String, delta: Long)(
      m: MutableMap[String, Long]
  )(implicit ev: Metrics.Source): Unit = {
    val fullName = s"$ev.$name"
    val newValue = m.get(fullName).map(_ + delta).getOrElse(delta)
    m.update(fullName, newValue)
  }

  private def set[A](name: String, value: A)(m: MutableMap[String, A]): Unit =
    m.update(name, value)

  override def incrementCounter(name: String, delta: Long)(implicit ev: Metrics.Source): F[Unit] =
    Sync[F].delay(incrementBy(name, delta)(counters))

  override def incrementSampler(name: String, delta: Long)(implicit ev: Metrics.Source): F[Unit] =
    Sync[F].delay(incrementBy(name, delta)(samplers))

  // no idea how to implement this properly
  override def sample(name: String)(implicit ev: Metrics.Source): F[Unit] = ???

  override def setGauge(name: String, value: Long)(implicit ev: Metrics.Source): F[Unit] =
    Sync[F].delay(set(name, value)(gauges))
  override def incrementGauge(name: String, delta: Long)(implicit ev: Metrics.Source): F[Unit] =
    Sync[F].delay(incrementBy(name, delta)(gauges))
  override def decrementGauge(name: String, delta: Long)(implicit ev: Metrics.Source): F[Unit] =
    Sync[F].delay(incrementBy(name, -delta)(gauges))
  override def record(name: String, value: Long, count: Long)(
      implicit ev: Metrics.Source
  ): F[Unit] =
    Sync[F].delay {
      val record     = Record(value, count)
      val recordsSeq = records.get(name).map(s => record :: s).getOrElse(List(record))
      set(name, recordsSeq)(records)
    }

  override def timer[A](name: String)(block: F[A])(implicit ev: Metrics.Source): F[A] = block
  override def timerS[A](name: String)(stream: fs2.Stream[F, A])(
      implicit ev: Metrics.Source
  ): fs2.Stream[F, A] = stream
}
