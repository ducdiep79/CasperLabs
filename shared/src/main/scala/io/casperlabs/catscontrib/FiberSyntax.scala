package io.casperlabs.catscontrib

import cats.effect.Concurrent
import cats.effect.implicits._
import cats.implicits._
import io.casperlabs.shared.Log

class FiberSyntax[F[_]: Concurrent: Log: MonadThrowable, A](fa: F[A]) {

  /** Forks execution into a fiber.
    * Logs any errors thrown in the fiber.
    * Return something we can wait upon until the fiber is finished. */
  def forkAndLog: F[F[A]] =
    fa.onError {
        case error =>
          Log[F].error(s"Error was thrown in the forked fiber: $error")
      }
      .start
      .map(_.join)

}
