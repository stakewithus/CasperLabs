package io.casperlabs.node

import java.io.File
import java.security.cert.X509Certificate

import cats.data.EitherT.{leftT, rightT}
import cats.syntax.applicativeError._
import cats.syntax.either._
import io.casperlabs.comm._
import io.casperlabs.comm.discovery.NodeIdentifier
import io.casperlabs.crypto.codec.Base16
import io.casperlabs.crypto.util.CertificateHelper
import io.casperlabs.node.configuration.Configuration
import io.casperlabs.shared.Log
import monix.eval.Task

import scala.util._

object NodeEnvironment {

  def create(conf: Configuration)(implicit log: Log[Task]): Effect[NodeIdentifier] =
    for {
      dataDir <- Task.delay(conf.server.dataDir.toFile).toEffect
      _       <- canCreateDataDir(dataDir)
      _       <- haveAccessToDataDir(dataDir)
      _       <- log.info(s"Using data dir: ${dataDir.getAbsolutePath}").toEffect
      _       <- hasCertificate(conf)
      _       <- hasKey(conf)
      name    <- name(conf)
    } yield NodeIdentifier(name)

  private def isValid(pred: Boolean, msg: String): Effect[Unit] =
    if (pred) Left[CommError, Unit](InitializationError(msg)).toEitherT
    else Right(()).toEitherT

  private def name(conf: Configuration): Effect[String] = {
    val certificate: Effect[X509Certificate] =
      Task
        .delay(CertificateHelper.fromFile(conf.tls.certificate.toFile))
        .attemptT
        .leftMap(e => InitializationError(s"Failed to read the X.509 certificate: ${e.getMessage}"))

    for {
      cert <- certificate
      pk   = cert.getPublicKey
      name <- certBase16(CertificateHelper.publicAddress(pk))
    } yield name
  }

  private def certBase16(maybePubAddr: Option[Array[Byte]]): Effect[String] =
    maybePubAddr match {
      case Some(bytes) => rightT(Base16.encode(bytes))
      case None =>
        leftT(
          InitializationError(
            "Certificate must contain a secp256r1 EC Public Key"
          )
        )
    }

  private def canCreateDataDir(dataDir: File): Effect[Unit] = isValid(
    !dataDir.exists() && !dataDir.mkdir(),
    s"The data dir must be a directory and have read and write permissions:\\n${dataDir.getAbsolutePath}"
  )

  private def haveAccessToDataDir(dataDir: File): Effect[Unit] = isValid(
    !dataDir.isDirectory || !dataDir.canRead || !dataDir.canWrite,
    s"The data dir must be a directory and have read and write permissions:\n${dataDir.getAbsolutePath}"
  )

  private def hasCertificate(conf: Configuration): Effect[Unit] = isValid(
    !conf.tls.certificate.toFile.exists(),
    s"Certificate file ${conf.tls.certificate} not found"
  )

  private def hasKey(conf: Configuration): Effect[Unit] = isValid(
    !conf.tls.key.toFile.exists(),
    s"Secret key file ${conf.tls.key} not found"
  )
}
